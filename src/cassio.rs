use super::*;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use std::thread::sleep;
use std::time::Duration;

// const HEADER : &str = "ENGINE-PROTOCOL ";
// const VERSION: &str = env!("CARGO_PKG_VERSION");
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

    fn getio(&mut self)
        -> Result<(&mut ChildStdin, &mut ChildStdout, &mut ChildStderr), String> {
        let ch = self.selectplayer()?;
        let fromerr = if let Some(engerr) = ch.stderr.as_mut() {
            engerr
        } else {
            return Err("failed to get error pipe..".to_string());
        };
        // let mut txt = String::new();
        // fromerr.read_to_string(&mut txt).unwrap();
        // println!("stderr:{txt}");
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
        Ok((toeng, fromeng, fromerr))
    }

    pub fn init(&mut self) -> Result<(), String> {
        match self.ply1.as_mut().unwrap().try_wait() {
            Ok(Some(es)) => {
                return Err(format!("player1 exit with {es}"));
            },
            Ok(_) => {
                // eprintln!("alive...")
            },
            Err(e) => {
                return Err(format!("Error:{e} in init()"))
            }
        }

        let (toeng, fromeng, _fromerr) = self.getio()?;

        if let Err(e) = toeng.write("ENGINE-PROTOCOL init\n".as_bytes()) {
            // let mut txt = String::new();
            // fromerr.read_to_string(&mut txt).unwrap();
            // println!("stderr:{txt}");
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
            print!("i:{i} buf:{buf:?}");
            sleep(Duration::from_millis(1));
        }
        Err(format!("unknown response ii: \"{buf}\""))
    }

    pub fn get_version(&mut self) -> Result<String, String> {
        if let Ok(Some(es)) = self.ply1.as_mut().unwrap().try_wait() {
            panic!("player1 exit with {es}");
        }

        let (toeng, fromeng, _fromerr) = self.getio()?;

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
        let (toeng, fromeng, _fromerr) = self.getio()?;

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
        let (toeng, fromeng, _fromerr) = self.getio()?;

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
        if let Ok(Some(es)) = self.ply1.as_mut().unwrap().try_wait() {
            panic!("player1 exit with {es}");
        }
        let (toeng, fromeng, _fromerr) = self.getio()?;
        let cmd = format!(
            "ENGINE-PROTOCOL endgame-search {obf} {alpha} {beta} {depth} {precision}\n");
    // eprintln!("cmd: {cmd}");
        if let Err(e) = toeng.write(
            cmd.as_bytes()) {
            return Err(e.to_string());
        }

        // let dur = Duration::from_millis(50);
        // let dur = Duration::from_millis(20);
        // let dur = Duration::from_millis(10);
        let dur = Duration::from_millis(1);
        sleep(dur);
        // eprint!("sleep");
// ENGINE-PROTOCOL endgame-search OOOOOO-OOOOOO-OOOOOOOOOOOOOOOOOOOOOOOOOOOXOOOOOOOOOOOOOOOOOOOOOO X -999 999 4 0
// ENGINE-PROTOCOL endgame-search OOOOOO-OOOOOO-OOOOOOOOOOOOOOOOOOOOOOOOOOOXOOOOOOOOOOOOOOOOOOOOOO X -999 999 4 0
        // read w/ a thread to imitate async reading. 
        // let (tx, rx) = std::sync::mpsc::channel::<String>();
        // let thread_read = std::thread::spawn(|| {
        //     loop {
        //         let buf = fromeng.take(300);
        //     }
        // });
        let no_bufreader = false;
        // let no_bufreader = true;
    if no_bufreader {
        let mut buf = String::default();
        let mut ba = [0u8 ; 1024];
        for _i in 0..1000 {
            let sz = match fromeng.read(&mut ba) {
                Ok(s) => s,
                Err(err) => {
                    panic!("err:{err} w/ {ba:?}");
                }
            };
            let line = unsafe {
                String::from_utf8_unchecked(ba[..sz].to_vec())
            };
            // fromeng.read_to_string(&mut line).unwrap();
            buf += &line.replace("\r\n", "\n");
// eprintln!("l:{line:?}");
            let eng = buf.split("\n").collect::<Vec<_>>();
// eprintln!("l:{:?}", eng[1]);
            let ready = "ready.";
            if eng[1] == ready {
            // if eng[1].starts_with(ready) {
// eprintln!("ret:{}", eng[0]);
                return Ok(eng[0].to_string());
            }
            sleep(dur);
        }
        Err(format!("unknown response ms: \"{buf}\""))
    } else {
        let mut bufreader = BufReader::new(fromeng);
        // eprintln!("BufReader");
        // read result
        let mut buf = String::default();
        // eprintln!("read_line");
        if let Err(e) = bufreader.read_line(&mut buf) {
            panic!("err:{e} for read_line.");
        }
        let ret = buf.trim().to_string();
        // eprintln!("readline {ret}");

        // read "ready."
        buf.clear();
        // eprintln!("read_line 2");
        bufreader.read_line(&mut buf).unwrap();
        if buf.replace("\r\n", "\n") == READY {
            return Ok(ret);
        }
        Err(format!("unknown response ms: \"{buf}\" \"{ret}\""))
    }
    }

    pub fn get_serach_infos(&mut self) {unimplemented!()}

    pub fn stop(&mut self) -> Result<(), String> {
        let (toeng, fromeng, _fromerr) = self.getio()?;

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
        let (toeng, fromeng, _fromerr) = self.getio()?;

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
        let (toeng, _fromeng, _fromerr) = self.getio()?;

        if let Err(e) = toeng.write("ENGINE-PROTOCOL quit\n".as_bytes()) {
            return Err(e.to_string());
        }

        sleep(Duration::from_millis(1));

        Ok(())
    }
}
