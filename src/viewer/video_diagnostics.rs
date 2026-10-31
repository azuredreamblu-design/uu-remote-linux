//! Opt-in, numeric-only video diagnostics. Never retain packets or desktop pixels.
use std::sync::{OnceLock,atomic::{AtomicU64,Ordering}};
use std::time::Instant;

pub fn enabled()->bool {*ENABLED.get_or_init(||std::env::var("UU_VIEWER_VIDEO_DIAGNOSTICS").is_ok_and(|v|v=="1"))}
static ENABLED:OnceLock<bool>=OnceLock::new();
pub const RX:usize=0; pub const FED:usize=1; pub const RGB:usize=2; pub const SHOWN:usize=3;
pub const AU_DROP:usize=4; pub const RGB_DROP:usize=5; pub const DRAIN_DROP:usize=6;
pub const GATE_DROP:usize=7; pub const ERRORS:usize=8; pub const RESETS:usize=9;
pub const PIPE:usize=10; pub const DRAW:usize=13; pub const TICK_GAP:usize=16; pub const PLI:usize=17;
const RTP_PACKETS:usize=18; const RTP_BYTES:usize=19; const RTP_FRAMES:usize=20; const RTP_MARKERS:usize=21;
const RTP_GAPS:usize=22; const RTP_LATE:usize=23; const RTP_STEPS:usize=24; const RTP_TICKS:usize=25;
const N:usize=26;
static COUNTERS:[AtomicU64;N]=[const{AtomicU64::new(0)};N];
static HOST_FPS:AtomicU64=AtomicU64::new(0);
static EPOCH:OnceLock<Instant>=OnceLock::new();
pub fn elapsed_ms()->u64 {EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64}

/// Track only forward RTP sequence/timestamp progress, before depacketization. Never log the
/// identifiers themselves. Gaps are observations, not final loss: late packets may repair them.
#[derive(Default)]
pub struct PacketProbe {seq:Option<u16>,timestamp:Option<u32>}
impl PacketProbe {
    fn observe(&mut self,seq:u16,timestamp:u32,marker:bool,bytes:usize)->[u64;8]{
        let mut d=[1,bytes as u64,0,u64::from(marker),0,0,0,0];
        if let Some(last)=self.seq {
            let step=seq.wrapping_sub(last);
            if step==0 || step>=32768 {d[5]=1;return d}
            d[4]=u64::from(step-1);
        }
        self.seq=Some(seq);
        match self.timestamp {
            None=>{self.timestamp=Some(timestamp);d[2]=1;},
            Some(last)=>{let step=timestamp.wrapping_sub(last);if step>0 && step<0x8000_0000 {
                self.timestamp=Some(timestamp);d[2]=1;d[6]=1;d[7]=u64::from(step);
            }},
        }
        d
    }
    pub fn packet(&mut self,seq:u16,timestamp:u32,marker:bool,bytes:usize){
        if !enabled(){return}
        for (i,value) in self.observe(seq,timestamp,marker,bytes).into_iter().enumerate(){
            COUNTERS[RTP_PACKETS+i].fetch_add(value,Ordering::Relaxed);
        }
    }
}
fn timing_fps(steps:u64,ticks:u64)->u64 {if ticks==0 {0}else{steps.saturating_mul(90000).saturating_add(ticks/2)/ticks}}
pub fn add(index:usize,value:u64){if enabled(){COUNTERS[index].fetch_add(value,Ordering::Relaxed);}}
pub fn tick_gap(previous:&mut Instant){let now=Instant::now();if enabled(){COUNTERS[TICK_GAP].fetch_max(now.duration_since(*previous).as_micros() as u64,Ordering::Relaxed);}*previous=now;}
pub struct Span(Option<(usize,Instant)>);
impl Span {pub fn new(index:usize)->Self {Self(enabled().then(||(index,Instant::now())))}}
impl Drop for Span {fn drop(&mut self){if let Some((i,start))=self.0 {
    let us=start.elapsed().as_micros() as u64;
    COUNTERS[i].fetch_add(1,Ordering::Relaxed);COUNTERS[i+1].fetch_add(us,Ordering::Relaxed);COUNTERS[i+2].fetch_max(us,Ordering::Relaxed);
}}}

#[derive(Default,Clone,PartialEq,Debug)]
pub struct Rates {pub rx:u32,pub fed:u32,pub decoded:u32,pub shown:u32,pub host_fps:Option<u32>,pub requested:u32}
fn rate(count:u64,secs:f64)->u32 {(count as f64/secs.max(0.001)).round().min(u32::MAX as f64) as u32}
fn read()->[u64;N] {std::array::from_fn(|i|if [PIPE+2,DRAW+2,TICK_GAP].contains(&i){COUNTERS[i].swap(0,Ordering::Relaxed)}else{COUNTERS[i].load(Ordering::Relaxed)})}
pub struct Reporter {last:[u64;N],at:Instant}
impl Reporter {
    pub fn new()->Self {let _=elapsed_ms();Self{last:read(),at:Instant::now()}}
    pub fn sample(&mut self,requested:u32,quality:u64,focused:bool,menu:bool)->Option<Rates>{
        if !enabled(){return None}
        let now=Instant::now();let secs=now.duration_since(self.at).as_secs_f64();let values=read();
        let d:[u64;N]=std::array::from_fn(|i|values[i].saturating_sub(self.last[i]));
        let host=HOST_FPS.load(Ordering::Relaxed);
        let out=Rates{rx:rate(d[RX],secs),fed:rate(d[FED],secs),decoded:rate(d[RGB],secs),shown:rate(d[SHOWN],secs),host_fps:u32::try_from(host).ok().filter(|v|*v>0),requested};
        // Pipe time is synchronous write/flush time, NOT FFmpeg's asynchronous decode duration.
        let elapsed=elapsed_ms();
        eprintln!("DECKPAD_VIDEO_SAMPLE interval_ms={} rx_au={} fed_au={} rgb={} shown={} au_drop={} rgb_drop={} drain_drop={} gate_drop={} errors={} resets={} pli={} pipe_avg_us={} pipe_max_us={} draw_avg_us={} draw_max_us={} tick_gap_max_us={} requested_fps={} host_fps={} focused={} menu={} elapsed_ms={} requested_quality={}",
            (secs*1000.).round() as u64,d[RX],d[FED],d[RGB],d[SHOWN],d[AU_DROP],d[RGB_DROP],d[DRAIN_DROP],d[GATE_DROP],d[ERRORS],d[RESETS],d[PLI],
            d[PIPE+1]/d[PIPE].max(1),values[PIPE+2],d[DRAW+1]/d[DRAW].max(1),values[DRAW+2],values[TICK_GAP],requested,host,u8::from(focused),u8::from(menu),elapsed,quality);
        // H.264/H.265 use a 90 kHz RTP clock (rtc::media_engine). Timing is observed cadence,
        // not a host configuration acknowledgement; losses/reordering can lower it too.
        eprintln!("DECKPAD_VIDEO_NETWORK elapsed_ms={} interval_ms={} rtp_packets={} rtp_bytes={} rtp_frames={} rtp_markers={} seq_gaps={} late_packets={} timestamp_steps={} timestamp_ticks={} rtp_timing_fps={}",
            elapsed,(secs*1000.).round() as u64,d[RTP_PACKETS],d[RTP_BYTES],d[RTP_FRAMES],d[RTP_MARKERS],d[RTP_GAPS],d[RTP_LATE],d[RTP_STEPS],d[RTP_TICKS],timing_fps(d[RTP_STEPS],d[RTP_TICKS]));
        self.last=values;self.at=now;Some(out)
    }
}

#[derive(Debug,PartialEq)]
struct CaptureSync {fps:u64,chroma:u64,width:u64,height:u64}
fn capture_sync(data:&[u8])->Option<CaptureSync>{
    // Message 25 CaptureSettingSync: field 1 fps, 3 chroma, 5/6 current output size.
    // Evidence: src/capture_setting.rs::sync_payload; missing/zero fps remains unknown.
    let body=super::tools::blob(data,25)?;
    Some(CaptureSync{fps:super::tools::uint(body,1),chroma:super::tools::uint(body,3),width:super::tools::uint(body,5),height:super::tools::uint(body,6)})
}
pub fn control(data:&[u8]){
    if !enabled() || data.len()>65536{return}
    if let Some(s)=capture_sync(data){
        if (1..=1000).contains(&s.fps){HOST_FPS.store(s.fps,Ordering::Relaxed);}
        eprintln!("DECKPAD_VIDEO_SYNC fps={} chroma={} width={} height={} elapsed_ms={}",s.fps,s.chroma,s.width,s.height,elapsed_ms());
    }
    if let Some(rpc)=super::tools::blob(data,22) {if let Some(reply)=super::tools::blob(rpc,2) {
        if let Ok(fields)=crate::protocol::decode_wire(reply){
            let errors=fields.iter().filter(|f|f.number==1).count();
            let request=super::tools::blob(rpc,1).map_or(0,|header|super::tools::uint(header,1));
            eprintln!("DECKPAD_VIDEO_REPLY capture_errors={errors} request_id={request} elapsed_ms={}",elapsed_ms());
        }
    }}
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn capture_sync_keeps_only_known_numbers(){
        let data=super::super::wire::envelope(1,25,&[8,60,24,3,40,128,10,48,208,5]);
        assert_eq!(capture_sync(&data),Some(CaptureSync{fps:60,chroma:3,width:1280,height:720}));
        assert_eq!(capture_sync(&super::super::wire::envelope(2,25,&[24,1])),Some(CaptureSync{fps:0,chroma:1,width:0,height:0}));
        assert_eq!(capture_sync(br#"{"token":"not-a-protobuf-message"}"#),None);
    }
    #[test] fn rates_use_elapsed_time_instead_of_assuming_one_second(){assert_eq!(rate(144,2.),72);assert_eq!(rate(0,0.),0);assert_eq!(rate(60,1.001),60);}
    #[test] fn packet_probe_tracks_rate_change_without_decode_or_render(){
        let mut p=PacketProbe::default();let mut seq=0u16;let mut timestamp=0u32;
        p.observe(seq,timestamp,true,100);seq+=1;
        for (frames,step) in [(144,625),(30,3000)] {
            let mut totals=[0u64;8];
            for _ in 0..frames {timestamp+=step;
                for marker in [false,true] {
                    let d=p.observe(seq,timestamp,marker,100);seq+=1;
                    for i in 0..8 {totals[i]+=d[i];}
                }
            }
            assert_eq!(totals,[frames*2,frames*200,frames,frames,0,0,frames,90000]);
            assert_eq!(timing_fps(totals[6],totals[7]),frames);
        }
    }
    #[test] fn packet_probe_handles_wrap_gap_late_and_duplicate(){
        let mut p=PacketProbe::default();p.observe(65534,u32::MAX-624,false,1);
        assert_eq!(p.observe(0,0,true,2),[1,2,1,1,1,0,1,625]);
        assert_eq!(p.observe(65535,u32::MAX-624,false,3),[1,3,0,0,0,1,0,0]);
        assert_eq!(p.observe(0,0,true,2),[1,2,0,1,0,1,0,0]);
        assert_eq!(p.observe(1,625,true,4),[1,4,1,1,0,0,1,625]);
        assert_eq!(timing_fps(0,0),0);
    }
}
