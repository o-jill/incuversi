use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdin, ChildStdout};
use std::thread::sleep;
use std::time::Duration;

const HEADER : &str = "ENGINE-PROTOCOL ";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const READY : &str = "ready.\n";


pub struct OthelloEngineProtocolServer {
    ply1 : Option<Child>,
    ply2 : Option<Child>,
    turn : i8,
}

impl OthelloEngineProtocolServer {
    pub fn new1(ch : Child) -> Self {
        OthelloEngineProtocolServer {
            ply1 : Some(ch),
            ply2 : None,
            turn : bitboard::NONE,
        }
    }

    pub fn new2(ch1 : Child, ch2 : Child) -> Self {
        OthelloEngineProtocolServer {
            ply1 : Some(ch1),
            ply2 : Some(ch2),
            turn : bitboard::NONE,
        }
    }

    pub fn setturn(&mut self, trn : i8) {self.turn = trn;}

    fn selectplayer(&mut self) -> Result<&mut Child, String> {
        if self.turn == bitboard::NONE {
            return Err("turn is NONE!".to_string());
        }

        Ok(if self.turn == bitboard::SENTE {
            self.ply1.as_mut().unwrap()
        } else {
            self.ply2.as_mut().unwrap()
        })
    }

    fn getio(&mut self) -> Result<(&mut ChildStdin, &mut ChildStdout), String> {
        let ch = self.selectplayer()?;
        let toeng = if let Some(toeng) = ch.stdin.as_mut() {
            toeng
        } else {
            return Err("failed  to get to-engine pipe..".to_string());
        };
        let fromeng = if let Some(fromeng) = ch.stdout.as_mut() {
            fromeng
        } else {
            return Err("failed  to get from-engine pipe..".to_string());
        };
        Ok((toeng, fromeng))
    }

    pub fn init(&mut self) -> Result<(), String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) = toeng.write("ENGINE-PROTOCOL init\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        for i in 0..100 {
            bufreader.read_line(&mut buf).unwrap();
            if buf.replace("\r\n", "\n") == READY {
                return Ok(())
            }
            if !buf.is_empty() {
                break;
            }
            println!("i:{i}");
        }
        Err(format!("unknown response ii: \"{buf}\""))
    }

    pub fn get_version(&mut self) -> Result<String, String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) =
                toeng.write("ENGINE-PROTOCOL get-version\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        bufreader.read_line(&mut buf).unwrap();
        let ret = buf.trim().to_string();
        buf.clear();
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(ret)
        }
        Err(format!("unknown response gv: \"{buf}\" \"{ret}\""))
    }

    pub fn new_position(&mut self) -> Result<(), String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) =
                toeng.write("ENGINE-PROTOCOL new-position\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(())
        }
        Err(format!("unknown response np: \"{buf}\""))
    }

    pub fn midgame_search(&mut self,
            obf : &str, alpha : f32, beta : f32, depth : u8, precision : i8)
            -> Result<String, String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) = toeng.write(
            format!(
                "ENGINE-PROTOCOL midgame-search {obf} {alpha} {beta} {depth} {precision}\n"
            ).as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        bufreader.read_line(&mut buf).unwrap();
        let ret = buf.trim().to_string();
        buf.clear();
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(ret)
        }
        Err(format!("unknown response ms: \"{buf}\" \"{ret}\""))
    }

    // pub fn midgame_search_thr(&mut self,
    //         obf : &str, alpha : f32, beta : f32, depth : u8, precision : i8)
    //          -> Result<String, String> {
    //     let (toeng, fromeng) = self.getio()?;

    //     if let Err(e) = toeng.write(
    //         format!(
    //             "ENGINE-PROTOCOL midgame-search {obf} {alpha} {beta} {depth} {precision}\n"
    //         ).as_bytes()) {
    //         return Err(e.to_string());
    //     }

    //     sleep(Duration::from_millis(1));

    //     let finished = Arc::new(AtomicBool::new(false));
    //     let finishthread = finished.clone();
    //     // let outfd = toeng.as_raw_fd();
    //     let thread = spawn(move || {
    //         // let mut toeng = unsafe {std::fs::File::from_raw_fd(outfd)};
    //         loop {
    //             for _i in 0..100 {
    //                 let fin = finishthread.load(Ordering::Relaxed);
    //                 if fin {return;}

    //                 std::thread::sleep(Duration::from_millis(10));
    //             }
    //             toeng.write("\n".as_bytes()).unwrap();
    //         }
    //     });
    //     let mut bufreader = BufReader::new(fromeng);
    //     let mut buf = String::default();
    //     let mut ret = String::default();
    //     loop {
    //         buf.clear();
    //         bufreader.read_line(&mut buf).unwrap();
    //         eprint!("recv:{buf}");
    //         let resp = buf.trim();
    //         if resp == "ok." {
    //             continue;
    //         } else if resp == "ready." {
    //             if ret.is_empty() {continue;}

    //             finished.store(true, Ordering::Relaxed);
    //             // drop(bufreader);
    //             thread.join().unwrap();
    //             let _ = toeng;
    //             return Ok(ret);
    //         } else if resp.is_empty() {
    //             // error
    //             finished.store(true, Ordering::Relaxed);
    //             // drop(bufreader);
    //             thread.join().unwrap();
    //             let _ = toeng;
    //             return Err(format!("unknown response mgs: \"{resp}\""));
    //         }
    //         ret = resp.to_string();
    //     }

    // }

    pub fn endgame_search(&mut self,
            obf : &str, alpha : f32, beta : f32, depth : u8, precision : i8)
            -> Result<String, String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) = toeng.write(
            format!(
                "ENGINE-PROTOCOL endgame-search {obf} {alpha} {beta} {depth} {precision}\n"
            ).as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        bufreader.read_line(&mut buf).unwrap();
        let ret = buf.trim().to_string();
        buf.clear();
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(ret)
        }
        Err(format!("unknown response ms: \"{buf}\" \"{ret}\""))
    }

    pub fn get_serach_infos(&mut self) {unimplemented!()}

    pub fn stop(&mut self) -> Result<(), String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) = toeng.write("ENGINE-PROTOCOL stop\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(())
        }
        Err(format!("unknown response sp: \"{buf}\""))
    }

    pub fn empty_hash(&mut self) -> Result<(), String> {
        let (toeng, fromeng) = self.getio()?;

        if let Err(e) =
                toeng.write("ENGINE-PROTOCOL empty-hash\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        let mut bufreader = BufReader::new(fromeng);
        let mut buf = String::default();
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(())
        }
        Err(format!("unknown response eh: \"{buf}\""))
    }

    pub fn quit(&mut self) -> Result<(), String> {
        let (toeng, _fromeng) = self.getio()?;

        if let Err(e) = toeng.write("ENGINE-PROTOCOL quit\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        Ok(())
    }
}
