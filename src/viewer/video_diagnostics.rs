//! Opt-in, numeric-only video diagnostics. Never retain packets or desktop pixels.
use std::sync::{OnceLock,atomic::{AtomicU64,Ordering}};
use std::time::Instant;

pub fn enabled()->bool {*ENABLED.get_or_init(||std::env::var("UU_VIEWER_VIDEO_DIAGNOSTICS").is_ok_and(|v|v=="1"))}
static ENABLED:OnceLock<bool>=OnceLock::new();
pub const RX:usize=0; pub const FED:usize=1; pub const RGB:usize=2; pub const SHOWN:usize=3;
pub const AU_DROP:usize=4; pub const RGB_DROP:usize=5; pub const DRAIN_DROP:usize=6;
pub const GATE_DROP:usize=7; pub const ERRORS:usize=8; pub const RESETS:usize=9;
pub const PIPE:usize=10; pub const DRAW:usize=13; pub const TICK_GAP:usize=16; pub const PLI:usize=17;
const N:usize=18;
static COUNTERS:[AtomicU64;N]=[const{AtomicU64::new(0)};N];
static HOST_FPS:AtomicU64=AtomicU64::new(0);
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
    pub fn new()->Self {Self{last:read(),at:Instant::now()}}
    pub fn sample(&mut self,requested:u32,focused:bool,menu:bool)->Option<Rates>{
        if !enabled(){return None}
        let now=Instant::now();let secs=now.duration_since(self.at).as_secs_f64();let values=read();
        let d:[u64;N]=std::array::from_fn(|i|values[i].saturating_sub(self.last[i]));
        let host=HOST_FPS.load(Ordering::Relaxed);
        let out=Rates{rx:rate(d[RX],secs),fed:rate(d[FED],secs),decoded:rate(d[RGB],secs),shown:rate(d[SHOWN],secs),host_fps:u32::try_from(host).ok().filter(|v|*v>0),requested};
        // Pipe time is synchronous write/flush time, NOT FFmpeg's asynchronous decode duration.
        eprintln!("DECKPAD_VIDEO_SAMPLE interval_ms={} rx_au={} fed_au={} rgb={} shown={} au_drop={} rgb_drop={} drain_drop={} gate_drop={} errors={} resets={} pli={} pipe_avg_us={} pipe_max_us={} draw_avg_us={} draw_max_us={} tick_gap_max_us={} requested_fps={} host_fps={} focused={} menu={}",
            (secs*1000.).round() as u64,d[RX],d[FED],d[RGB],d[SHOWN],d[AU_DROP],d[RGB_DROP],d[DRAIN_DROP],d[GATE_DROP],d[ERRORS],d[RESETS],d[PLI],
            d[PIPE+1]/d[PIPE].max(1),values[PIPE+2],d[DRAW+1]/d[DRAW].max(1),values[DRAW+2],values[TICK_GAP],requested,host,u8::from(focused),u8::from(menu));
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
        eprintln!("DECKPAD_VIDEO_SYNC fps={} chroma={} width={} height={}",s.fps,s.chroma,s.width,s.height);
    }
    if let Some(reply)=super::tools::blob(data,22).and_then(|rpc|super::tools::blob(rpc,2)) {
        if let Ok(fields)=crate::protocol::decode_wire(reply){
            let errors=fields.iter().filter(|f|f.number==1).count();
            eprintln!("DECKPAD_VIDEO_REPLY capture_errors={errors}");
        }
    }
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
}
