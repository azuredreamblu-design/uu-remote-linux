use super::media::Latest;
use std::sync::{Arc, atomic::{AtomicBool,AtomicU32,Ordering}};
#[derive(Clone)]
pub struct Desktop { pub units:Latest<super::media::AccessUnit>, pub pli:Arc<AtomicBool>, pub reset:Arc<AtomicBool>, pub ssrc:Arc<AtomicU32> }
impl Default for Desktop {
    // A bounded burst allowance, not a playback delay: each tick consumes queued units immediately.
    // Eight units cover a short UI redraw at 144 fps; persistent overload still requests recovery.
    fn default()->Self {Self{units:Latest::new(8),pli:Arc::new(AtomicBool::new(false)),reset:Arc::new(AtomicBool::new(false)),ssrc:Arc::new(AtomicU32::new(0))}}
}
/// A startup timeout must not terminate a desktop that has already established a usable path.
#[derive(Default)]
pub struct StartupGuard {established:bool}
impl StartupGuard {
    /// Returns true exactly once, so preferences are restored once even if echo arrives later.
    pub fn check(&mut self,ready:bool,elapsed:std::time::Duration)->Result<bool,String>{
        if self.established {return Ok(false)}
        if ready {self.established=true;return Ok(true)}
        if elapsed>=std::time::Duration::from_secs(30){return Err("Desktop startup timed out".into())}
        Ok(false)
    }
}
/// `content` is the source picture size inside the (possibly padded) RGB canvas; None = full frame.
pub struct Frame { pub width:u32, pub height:u32, pub rgb:Vec<u8>, pub content:Option<(u32,u32)> }
pub trait Decoder { fn set_codec(&mut self,codec:super::media::Codec)->Result<(),String>; fn decode(&mut self, au:&[u8])->Result<Vec<Frame>,String>; fn reset(&mut self);
    /// Frames finished since the last call, without new input; an asynchronous decoder
    /// returns a picture here instead of holding it until the next access unit arrives.
    fn poll(&mut self)->Vec<Frame> {Vec::new()} }
pub trait FrameSink { fn present(&mut self,frame:&Frame)->Result<(),String>; }
/// Counts only (access units, presented frames, decode errors) for the periodic stats line.
pub static STATS:[std::sync::atomic::AtomicU64;3]=[std::sync::atomic::AtomicU64::new(0),std::sync::atomic::AtomicU64::new(0),std::sync::atomic::AtomicU64::new(0)];
fn wants_key_frame(units:u64,presented:u64)->bool {presented==0 && units%30==0}
/// True when an Annex-B access unit carries a picture the decoder can restart from: an IDR/IRAP,
/// an H.264 recovery-point SEI or an H.264 I slice (some hosts answer PLI without an IDR, #136).
pub fn is_key_frame(codec:super::media::Codec,au:&[u8])->bool {
    au.windows(4).enumerate().filter(|(_,w)|w[..3]==[0,0,1]).any(|(i,w)|match codec {
        super::media::Codec::H264=>match w[3]&0x1f {5=>true,6=>au.get(i+4)==Some(&6),1=>h264_i_slice(&au[i+4..]),_=>false},
        super::media::Codec::H265=>(16..=21).contains(&((w[3]>>1)&0x3f)),
    })
}
/// Reads first_mb_in_slice and slice_type (both ue(v)) from a slice header; I = 2 or 7.
fn h264_i_slice(rbsp:&[u8])->bool {
    let mut bit=0usize;
    let mut ue=||->Option<u32>{
        let mut zeros=0;
        while (rbsp.get(bit/8)?>>(7-bit%8))&1==0 {zeros+=1;bit+=1;if zeros>31 {return None}}
        bit+=1;let mut v=0u32;
        for _ in 0..zeros {v=(v<<1)|u32::from((rbsp.get(bit/8)?>>(7-bit%8))&1);bit+=1;}
        Some((1u32<<zeros)-1+v)
    };
    ue().is_some() && matches!(ue(),Some(2|7))
}
/// After a local reset, holds back pictures until a restart point arrives, but never longer
/// than `limit`: a host that ignores PLI must not leave the window blank (#136).
pub struct KeyGate { since:Option<std::time::Instant>, limit:std::time::Duration }
impl KeyGate {
    pub fn new()->Self {Self{since:None,limit:std::time::Duration::from_millis(1500)}}
    pub fn arm(&mut self) {self.since.get_or_insert_with(std::time::Instant::now);}
    /// False = drop this unit and keep asking for a key frame.
    pub fn admit(&mut self,codec:super::media::Codec,au:&[u8])->bool {
        let Some(since)=self.since else {return true};
        let timed_out=since.elapsed()>=self.limit;
        if timed_out || is_key_frame(codec,au) {
            if timed_out {log::debug!("Viewer desktop: no key frame within {:?} after reset; decoding on",self.limit);}
            self.since=None;true
        } else {false}
    }
}
/// Shows the newest frame a decoder finished between access units.
pub fn drain(decoder:&mut dyn Decoder,sink:&mut dyn FrameSink)->Result<(),String> {
    let mut frames=decoder.poll();
    super::video_diagnostics::add(super::video_diagnostics::DRAIN_DROP,frames.len().saturating_sub(1) as u64);
    if let Some(frame)=frames.pop() {present(sink,&frame)?;}
    Ok(())
}
pub fn display(decoder:&mut dyn Decoder,sink:&mut dyn FrameSink,au:&[u8],codec:super::media::Codec,pli:&AtomicBool)->Result<(),String> {
    for frame in decode(decoder,au,codec,pli)? {present(sink,&frame)?;}
    Ok(())
}
fn present(sink:&mut dyn FrameSink,frame:&Frame)->Result<(),String>{
    sink.present(frame)?;
    super::video_diagnostics::add(super::video_diagnostics::SHOWN,1);
    // Local acceptance aid; preserve the existing opt-in image dump behavior.
    if STATS[1].fetch_add(1,Ordering::Relaxed)==std::env::var("UU_VIEWER_DUMP_FRAME_INDEX").ok().and_then(|v|v.parse().ok()).unwrap_or(4u64) { if let Some(path)=std::env::var_os("UU_VIEWER_DUMP_FRAME") {
        let mut ppm=format!("P6\n{} {}\n255\n",frame.width,frame.height).into_bytes();ppm.extend_from_slice(&frame.rgb);let _=std::fs::write(path,ppm);
    }}
    Ok(())
}
fn decode(decoder:&mut dyn Decoder,au:&[u8],codec:super::media::Codec,pli:&AtomicBool)->Result<Vec<Frame>,String> {
    let units=STATS[0].fetch_add(1,Ordering::Relaxed)+1;
    super::video_diagnostics::add(super::video_diagnostics::FED,1);
    // A later session can join a clean stream with no IDR in flight (#124): keep asking until a frame shows.
    if wants_key_frame(units,STATS[1].load(Ordering::Relaxed)) {
        pli.store(true,Ordering::Release);
        if units==300 { log::warn!("Viewer desktop: no decodable frame after {units} access units; requesting key frames (another session may hold the host)"); }
    }
    // Local diagnostic: UU_VIEWER_DUMP_H264=path appends the first 300 access units (owner's own stream).
    if codec==super::media::Codec::H264 && STATS[0].load(Ordering::Relaxed)<=300 { if let Some(path)=std::env::var_os("UU_VIEWER_DUMP_H264") {
        use std::io::Write; if let Ok(mut f)=std::fs::OpenOptions::new().create(true).append(true).open(path){let _=f.write_all(au);}
    }}
    match decoder.decode(au) {
        Ok(frames)=>Ok(frames),
        Err(_)=>{super::video_diagnostics::add(super::video_diagnostics::ERRORS,1);STATS[2].fetch_add(1,Ordering::Relaxed);decoder.reset();pli.store(true,Ordering::Release);Ok(Vec::new())},
    }
}

fn keep_latest(slot:&mut Option<Frame>,frame:Frame){
    if slot.replace(frame).is_some(){super::video_diagnostics::add(super::video_diagnostics::DRAIN_DROP,1);}
}

/// Decode a bounded FIFO batch, then present only the newest decoded picture. Dropping compressed
/// P-frames destroys references; skipping already-decoded RGB pictures does not. One AU per 8 ms
/// tick capped consumption at 125/s: a real Deck receiving 144/s repeatedly overflowed and gated
/// ~120 units/s while waiting for a key frame. Never tie compressed consumption to presentation.
pub fn pump(desktop:&Desktop,decoder:&mut dyn Decoder,sink:&mut dyn FrameSink,gate:&mut KeyGate)->Result<Option<super::media::Codec>,String>{
    let started=std::time::Instant::now();let mut newest=None;let mut codec=None;
    // Limit work to preserve input polling. This bounds calls; a synchronous decoder call itself
    // can still block on FFmpeg and is measured separately by the pipe-time diagnostics.
    for _ in 0..8 {
        let Some(au)=desktop.units.pop() else{break};
        if desktop.reset.swap(false,Ordering::AcqRel){
            super::video_diagnostics::add(super::video_diagnostics::RESETS,1);
            decoder.reset();gate.arm();
        }
        if !gate.admit(au.codec,&au.data){
            super::video_diagnostics::add(super::video_diagnostics::GATE_DROP,1);
            desktop.pli.store(true,Ordering::Release);
        }else{
            decoder.set_codec(au.codec)?;codec=Some(au.codec);
            for frame in decode(decoder,&au.data,au.codec,&desktop.pli)? {keep_latest(&mut newest,frame);}
        }
        if started.elapsed()>=std::time::Duration::from_millis(4){break}
    }
    // Also collect asynchronous FFmpeg output on idle ticks, without waiting for another AU.
    for frame in decoder.poll(){keep_latest(&mut newest,frame);}
    if let Some(frame)=newest {present(sink,&frame)?;}
    Ok(codec)
}
#[cfg(test)] mod tests {
    #[test] fn playing_desktop_survives_startup_deadline_and_restores_only_once(){
        use std::time::Duration;
        let mut guard=super::StartupGuard::default();let mut restores=0;
        // Owner trace: picture starts at ~8 s, no matching echo, healthy video through 30 s.
        for second in 0..=120 {
            restores+=usize::from(guard.check(second>=8,Duration::from_secs(second)).unwrap());
        }
        assert_eq!(restores,1);
        // A later transient control loss is not a fresh startup timeout.
        assert!(!guard.check(false,Duration::from_secs(121)).unwrap());
        let mut silent=super::StartupGuard::default();
        assert!(!silent.check(false,Duration::from_millis(29999)).unwrap());
        assert_eq!(silent.check(false,Duration::from_secs(30)).unwrap_err(),"Desktop startup timed out");
    }
    #[test] fn key_frames_by_nal_type(){
        use crate::viewer::media::Codec;
        assert!(super::is_key_frame(Codec::H264,&[0,0,0,1,0x67,1,0,0,0,1,0x68,2,0,0,0,1,0x65,3]));
        assert!(!super::is_key_frame(Codec::H264,&[0,0,0,1,0x61,1]));
        assert!(super::is_key_frame(Codec::H265,&[0,0,0,1,0x40,1,0,0,1,0x26,1,3]));
        assert!(!super::is_key_frame(Codec::H265,&[0,0,0,1,0x02,1]));
        // I slice (first_mb=0 '1', slice_type=7 '0001000') and P slice (slice_type=5 '00110').
        assert!(super::is_key_frame(Codec::H264,&[0,0,0,1,0x41,0b1000_1000,0]));
        assert!(!super::is_key_frame(Codec::H264,&[0,0,0,1,0x41,0b1001_1000]));
        // Recovery-point SEI (payload type 6).
        assert!(super::is_key_frame(Codec::H264,&[0,0,0,1,0x06,0x06,1,0x80]));
    }
    #[test] fn key_gate_waits_then_gives_up(){
        use crate::viewer::media::Codec;
        let mut g=super::KeyGate::new();let p=[0,0,0,1,0x41,0b1001_1000];
        assert!(g.admit(Codec::H264,&p));
        g.arm();assert!(!g.admit(Codec::H264,&p));
        assert!(g.admit(Codec::H264,&[0,0,0,1,0x65,0x88]));assert!(g.admit(Codec::H264,&p));
        g.limit=std::time::Duration::ZERO;g.arm();assert!(g.admit(Codec::H264,&p));
    }
    use super::*;
    struct Mock(bool);impl Decoder for Mock {fn set_codec(&mut self,_:crate::viewer::media::Codec)->Result<(),String>{Ok(())} fn decode(&mut self,_:&[u8])->Result<Vec<Frame>,String>{if self.0{Err("synthetic".into())}else{Ok(vec![Frame{width:1,height:1,rgb:vec![0;3],content:None}])}} fn reset(&mut self){self.0=false;}}
    struct Sink(usize);impl FrameSink for Sink{fn present(&mut self,f:&Frame)->Result<(),String>{assert_eq!(f.rgb.len(),3);self.0+=1;Ok(())}}
    #[test] fn requests_key_frames_until_the_first_frame(){
        assert_eq!((1..=90).filter(|&n|wants_key_frame(n,0)).count(),3);
        assert!(!(1..=90).any(|n|wants_key_frame(n,1)));
    }
    #[test] fn decode_error_requests_pli_and_recovers(){let mut d=Mock(true);let mut s=Sink(0);let p=AtomicBool::new(false);display(&mut d,&mut s,&[],crate::viewer::media::Codec::H264,&p).unwrap();assert!(p.load(Ordering::Acquire));display(&mut d,&mut s,&[],crate::viewer::media::Codec::H264,&p).unwrap();assert_eq!(s.0,1);}

    fn numbered_au(n:u32,key:bool)->super::super::media::AccessUnit{
        let mut data=vec![0,0,0,1,if key {0x65}else{0x61}];data.extend(n.to_le_bytes());
        super::super::media::AccessUnit{codec:super::super::media::Codec::H264,data}
    }
    fn numbered_frame(n:u32)->Frame{Frame{width:1,height:1,rgb:vec![n as u8,0,0],content:None}}
    #[derive(Default)] struct RecordingDecoder {fed:Vec<u32>,resets:usize,pending:Vec<Frame>}
    impl Decoder for RecordingDecoder {
        fn set_codec(&mut self,_:super::super::media::Codec)->Result<(),String>{Ok(())}
        fn reset(&mut self){self.resets+=1;}
        fn decode(&mut self,au:&[u8])->Result<Vec<Frame>,String>{let n=u32::from_le_bytes(au[5..9].try_into().unwrap());self.fed.push(n);Ok(vec![numbered_frame(n)])}
        fn poll(&mut self)->Vec<Frame>{std::mem::take(&mut self.pending)}
    }
    #[derive(Default)] struct RecordingSink(Vec<u8>);
    impl FrameSink for RecordingSink {fn present(&mut self,frame:&Frame)->Result<(),String>{self.0.push(frame.rgb[0]);Ok(())}}

    #[test] fn receives_144_per_second_with_125_input_ticks_without_compressed_drops(){
        // Virtual time: eight seconds, 144 incoming access units/s, one input tick every 8 ms.
        // The old one-pop policy overflows even if its queue is enlarged to eight slots.
        let desktop=Desktop::default();let old=Latest::new(8);
        let mut decoder=RecordingDecoder::default();let mut sink=RecordingSink::default();let mut gate=KeyGate::new();
        let(mut received,mut old_drops,mut new_drops)=(0,0,0);
        for tick in 1..=1000 {
            let due=tick*144/125;
            while received<due {
                received+=1;
                old_drops+=usize::from(old.push(numbered_au(received,false)));
                new_drops+=usize::from(desktop.units.push(numbered_au(received,false)));
            }
            let _=old.pop();
            pump(&desktop,&mut decoder,&mut sink,&mut gate).unwrap();
        }
        // One final drain tolerates a scheduler preemption at the time-budget boundary.
        pump(&desktop,&mut decoder,&mut sink,&mut gate).unwrap();
        assert!(old_drops>100,"the fixture must reproduce the old sustained overflow");
        assert_eq!(new_drops,0);
        assert_eq!(decoder.fed,(1..=1152).collect::<Vec<_>>(),"every compressed reference arrives in FIFO order");
        assert_eq!(decoder.resets,0);
        assert!(sink.0.len()<=1001,"present at most once per tick, not once per access unit");
    }
    #[test] fn batch_presents_newest_rgb_and_idle_tick_polls_async_output(){
        let desktop=Desktop::default();for n in 1..=3{desktop.units.push(numbered_au(n,false));}
        let mut decoder=RecordingDecoder::default();decoder.pending.push(numbered_frame(4));
        let mut sink=RecordingSink::default();let mut gate=KeyGate::new();
        pump(&desktop,&mut decoder,&mut sink,&mut gate).unwrap();
        assert_eq!(decoder.fed,vec![1,2,3]);assert_eq!(sink.0,vec![4]);
        decoder.pending.push(numbered_frame(5));
        pump(&desktop,&mut decoder,&mut sink,&mut gate).unwrap();assert_eq!(sink.0,vec![4,5]);
    }
    #[test] fn actual_compressed_loss_still_waits_for_recovery(){
        let desktop=Desktop::default();desktop.reset.store(true,Ordering::Release);
        for (n,key) in [(1,false),(2,false),(3,true),(4,false)]{desktop.units.push(numbered_au(n,key));}
        let mut decoder=RecordingDecoder::default();let mut sink=RecordingSink::default();let mut gate=KeyGate::new();
        pump(&desktop,&mut decoder,&mut sink,&mut gate).unwrap();
        assert_eq!(decoder.fed,vec![3,4]);assert_eq!(decoder.resets,1);assert_eq!(sink.0,vec![4]);assert!(desktop.pli.load(Ordering::Acquire));
    }
}
