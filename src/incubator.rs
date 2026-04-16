use super::*;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::io::{BufRead, BufReader};
use std::fs::OpenOptions;
use std::sync::mpsc;
use std::path::PathBuf;

pub struct Incubator {
    kifudir : Vec<String>,
    log : std::fs::File,
    mate : u32,
    // matefiles : String,
    mode : argument::Mode,
    multibar : MultiProgress,
    outdir : String,
    ruversi_config : String,
    show_progressbar : bool,
    verbose : bool,
}

impl std::fmt::Display for Incubator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

fn format_log_path(txt : &Option<String>) -> String {
    let strdt = Utc::now().format("%Y%m%d%H%M%S").to_string();
    if let Some(path) = txt {
        path.replace("<DATETIME>", &strdt)
    } else {
        if cfg!(target_os="windows") {
            String::from("nul")
        } else {
            String::from("/dev/null")
        }
    }
}

impl From<argument::Arg> for Incubator {
    fn from(arg : argument::Arg) -> Self {
        let path = format_log_path(&arg.log);
        let log = match std::fs::File::create(path) {
        Ok(f) => {f},
        Err(e) => {panic!("{e}");},
        };

        let mode = arg.md;
        let kifudir = arg.kifudir;
        let outdir = arg.output.unwrap_or(".".to_string());
        // let matefiles = arg.mate_file.unwrap_or(String::new()).clone();
        let ruversi_config = arg.ru_config.unwrap_or_default();
        let mate = arg.mate;
        let verbose = arg.verbose;

        Self {
            kifudir,
            log,
            mate,
            // matefiles,
            mode,
            multibar : MultiProgress::new(),
            outdir,
            ruversi_config,
            show_progressbar : !arg.no_progressbar,
            verbose,
        }
    }
}

impl Incubator {
    fn run_kifu(&mut self) -> Result<(), std::io::Error> {
        if self.mate < 3 || 60 <= self.mate {
            panic!("self.mate < 3 || 60 <= self.mate");
        }

        let dest_file = format!("mate{}.txt", self.mate - 1);
        if std::path::Path::new(&dest_file).exists() {
            panic!("{dest_file} exists!");
        }

        // const RELY_ON_RUVERSI : bool = true;
        const RELY_ON_RUVERSI : bool = false;
        if RELY_ON_RUVERSI && self.mate == 3 {
            return self.extract_mate3();
        }

        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(
                ProgressBar::new(self.kifudir.len() as u64 + 1));
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let show_path = self.verbose;
        if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        for d in self.kifudir.iter() {
            let pbchild = if self.show_progressbar {
                let pb = self.multibar.add(ProgressBar::new(7));
                    // load, dedup, extract, dedup, augmentation, dedup, store
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}]{wide_bar}[{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("📔📖📕"));
                pb.set_message("loading kifu...");
                Some(pb)
            } else {
                None
            };
            let mut boards = 
                    data_loader::loadkifu_for_mate(
                        &data_loader::findfiles(&format!("./{d}")),
                        d, self.mate, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 1

            data_loader::dedupboards(&mut boards, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 2

            // ruversiに展開してもらう
            let pbgrandchild = if self.show_progressbar {
                let pb = self.multibar.add(
                ProgressBar::new(boards.len() as u64));
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("🥚🐔🐤"));
                Some(pb)
            } else {
                None
            };
            let mut rr = ruversirunner::RuversiRunner::from_config(
                &std::path::PathBuf::from(
                    self.ruversi_config.clone())).unwrap();
            rr.set_verbose(self.verbose);
            let mut mates = boards.iter().flat_map(|(ban, _, _, _)| {
                if !ban.is_last_n(self.mate) {panic!("!ban.is_last_n({})", self.mate);}
                match rr.run_children(&ban.to_string()) {
                    Err(msg) => {panic!("{msg}")},
                    Ok(ban) => {
                        if let Some(pb) = &pbgrandchild {pb.inc(1);}
                        ban
                    },
                }
            }).collect::<Vec<_>>();
            if let Some(pb) = &pbchild {pb.inc(1);}  // 3
            if let Some(pb) = &pbgrandchild {
                pb.finish();
                self.multibar.remove(pb);
            }
            if mates.is_empty() {panic!("mates: {}", mates.len());}

            data_loader::dedupboards(&mut mates, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 4
            if mates.is_empty() {panic!("mates: {}", mates.len());}

            // augmentation
            const AUGMENTATION_KIFU : bool = false;
            let mates = if AUGMENTATION_KIFU {
                let mut newmates = mates.iter().flat_map(|(ban, fsb, fsw, score)| {
                    ban.rotated_mirrored_fixed(*fsb, *fsw, *score)
                }).collect::<Vec<_>>();
                if let Some(pb) = &pbchild {pb.inc(1);}  // 5

                data_loader::dedupboards(&mut newmates, &mut self.log, show_path);
                newmates
            } else {
                if let Some(pb) = &pbchild {pb.inc(1);}  // 5
                mates
            };
            if let Some(pb) = &pbchild {pb.inc(1);}  // 6

            // write to a file.
            let n1 = self.mate - 1;
            let text = format!("# {d}\n")
                + &mates.iter().filter_map(|(ban, _, _, score)| {
                if ban.is_last_n(n1) {
                    Some(format!("{ban},{score}\n"))
                } else {
                    // PASSだとこっちに来る。
                    // panic!("{ban} != {n1}");
                    None
                }
            }).collect::<Vec<String>>().join("");
            {
                let mut f = OpenOptions::new()
                    .create(true).append(true).open(&dest_file).unwrap();
                f.write_all(text.as_bytes()).unwrap();
            }

            if let Some(pb) = &pbchild {
                pb.inc(1);  // 7
                pb.finish();
                // self.multibar.remove(pb);
            }
            if let Some(pb ) = &pbtop {pb.inc(1);}
        }

        if let Some(pb ) = &pbtop {
            pb.finish_with_message("done!");
        }
        Ok(())
    }

    pub fn extract_mate3(&mut self) -> Result<(), std::io::Error> {
        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(ProgressBar::new(6));
            // kifudir, load, dedup, extract, dedup, store
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let pbchild = if self.show_progressbar {
            let pb = self.multibar.add(
            ProgressBar::new(self.kifudir.len() as u64 + 1));
            pb.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}]{wide_bar}[{eta_precise}] {pos}/{len} {msg}").unwrap()
                .progress_chars("🪵🪓🌴"));
            pb.set_message("loading kifu...");
            Some(pb)
        } else {
            None
        };
        let show_path = false;
        let mut boards = self.kifudir.iter().flat_map(
            |d| {
                let ret = data_loader::loadkifu_for_mate(
                    &data_loader::findfiles(&format!("./{d}")),
                    d, 3, &mut self.log, show_path);
                if let Some(pb) = &pbchild {pb.inc(1);}
                ret
            }).collect();
        if let Some(pb ) = &pbtop {pb.inc(1);}  // 1

        data_loader::dedupboards(&mut boards, &mut self.log, show_path);
        if let Some(pb ) = &pbtop {pb.inc(1);}  // 2

        // write to a file.
        let dest = "mate2.txt";
        let mut f = std::fs::File::create(dest).unwrap();
        for (ban, _, _, score) in boards {
            if !ban.is_last_n(2) {
                continue;
            }

            f.write_all(format!("{ban},{score}\n").as_bytes()).unwrap();
        }

        if let Some(pb ) = &pbtop {
            pb.inc(1);  // 3
            pb.finish_with_message("done!");
        }

        Ok(())
    }

    fn run_mate(&mut self) -> Result<(), std::io::Error> {
        if self.mate < 3 || 60 <= self.mate {
            panic!("self.mate < 3 || 60 <= self.mate");
        }

        let dest_file = format!("mate{}.txt", self.mate - 1);
        // if std::path::Path::new(&dest_file).exists() {
        //     panic!("{dest_file} exists!");
        // }

        // const RELY_ON_RUVERSI : bool = true;
        const RELY_ON_RUVERSI : bool = false;
        if RELY_ON_RUVERSI && self.mate == 3 {
            return self.extract_mate3();
        }

        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(
                ProgressBar::new(self.kifudir.len() as u64 + 1));
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let show_path = false;
        if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        for d in self.kifudir.iter() {
            let pbchild = if self.show_progressbar {
                let pb = self.multibar.add(ProgressBar::new(7));
                    // load, dedup, extract, dedup, augmentation, dedup, store
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}]{wide_bar}[{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("🪵🪓🌴"));
                pb.set_message("loading kifu...");
                Some(pb)
            } else {
                None
            };
            let files = data_loader::findfiles(&format!("./{d}")).iter().map(
                    |fname| format!("{d}/{fname}")).collect::<Vec<String>>();
            let mut boards = files.iter().flat_map(|path| {
                    data_loader::load_mates(path, self.mate).unwrap()
                }).collect();
            if let Some(pb) = &pbchild {pb.inc(1);}  // 1

            data_loader::dedupboards(&mut boards, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 2

            // ruversiに展開してもらう
            let pbgrandchild = if self.show_progressbar {
                let pb = self.multibar.add(
                ProgressBar::new(boards.len() as u64));
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("📗📖📓"));
                Some(pb)
            } else {
                None
            };
            let mut rr = ruversirunner::RuversiRunner::from_config(
                &std::path::PathBuf::from(
                    self.ruversi_config.clone())).unwrap();
            rr.set_verbose(self.verbose);
            let mut mates = boards.iter().flat_map(|(ban, _, _, _)| {
                if !ban.is_last_n(self.mate) {panic!("!ban.is_last_n({})", self.mate);}
                // rr.set_verbose(true);
                match rr.run_children(&ban.to_string()) {
                    Err(msg) => {panic!("{msg}")},
                    Ok(ban) => {
                        if let Some(pb) = &pbgrandchild {pb.inc(1);}
                        ban
                    },
                }
            }).collect::<Vec<_>>();
            if let Some(pb) = &pbchild {pb.inc(1);}  // 3
            if let Some(pb) = &pbgrandchild {
                pb.finish();
                self.multibar.remove(pb);
            }
            if mates.is_empty() {panic!("mates: {}", mates.len());}

            data_loader::dedupboards(&mut mates, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 4

            // augmentation
            const AUGMENTATION_MATE : bool = false;
            let mates = if AUGMENTATION_MATE {
                let mut newmates = mates.iter().flat_map(|(ban, fsb, fsw, score)| {
                    ban.rotated_mirrored_fixed(*fsb, *fsw, *score)
                }).collect::<Vec<_>>();
                if let Some(pb) = &pbchild {pb.inc(1);}  // 5
                data_loader::dedupboards(&mut newmates, &mut self.log, show_path);
                newmates
            } else {
                if let Some(pb) = &pbchild {pb.inc(1);}  // 5
                mates
            };
            if let Some(pb) = &pbchild {pb.inc(1);}  // 6
            if mates.is_empty() {panic!("mates: {}", mates.len());}

            // write to a file.
            let n1 = self.mate - 1;
            let text = files.join("\n# ") + "\n"
                + &mates.iter().filter_map(|(ban, _, _, score)| {
                if ban.is_last_n(n1) {
                    Some(format!("{ban},{score}\n"))
                } else {
                    // PASSだとこっちに来る。
                    // panic!("{ban} != {n1}");
                    None
                }
            }).collect::<Vec<String>>().join("");
            {
                let mut f = OpenOptions::new()
                    .create(true).append(true).open(&dest_file).unwrap();
                f.write_all(text.as_bytes()).unwrap();
            }

            if let Some(pb) = &pbchild {
                pb.inc(1);  // 7
                pb.finish();
                // self.multibar.remove(pb);
            }
            if let Some(pb ) = &pbtop {pb.inc(1);}
        }

        if let Some(pb ) = &pbtop {
            pb.finish_with_message("done!");
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        match self.mode {
            argument::Mode::Kifu => {
                self.run_kifu()
            },
            argument::Mode::Mate => {
                self.run_mate()
            },
            argument::Mode::Spread => {
                self.run_spread()
            },
            argument::Mode::Dedup => {
                self.run_dedup()
            },
            argument::Mode::Shorten => {
                self.run_shorten()
            },
            argument::Mode::Validate => {
                self.run_validate()
            },
            _ => {
                Ok(())
            },
        }
    }

    /// 受け取った文字列を残りのマス毎にファイルに分けて出力する。
    /// 
    /// # Arguments
    /// - rx
    ///   文字列受信チャンネル。emptyデータを受信すると関数を抜けます。
    /// - outdir
    ///   ファイルの出力ディレクトリ
    fn store_rfen_thread(rx : std::sync::mpsc::Receiver<String>, outdir : &PathBuf, suffix : &str) {
        let mut buf = vec![String::new() ; 64];
        const THREASHOLD_BYTES : usize = 10 * 1024;
        loop {
            match rx.recv() {
                Ok(lines) => {
                    if lines.is_empty() {
                        for (n,b) in buf.iter().enumerate() {
                            if b.is_empty() {continue;}

                            let mut dest_file = outdir.clone();
                            dest_file.push(format!("mate{n}_{suffix}.txt"));
                            let mut f = OpenOptions::new()
                                .create(true).append(true).open(dest_file).unwrap();
                            f.write_all(buf[n].as_bytes()).unwrap();
                        }
                        return;
                    }

                    for line in lines.split("\n") {
                        let elem = line.split(",").collect::<Vec<_>>();
                        if elem.len() < 2 {continue;}

                        let n = match bitboard::count_empty_cells(elem[0]) {
                                Ok(n) => {n as usize},
                                Err(msg) => {
                                    panic!("{msg}");
                                },
                            };
                        buf[n] += line;
                        buf[n] += "\n";
                    }
                    // 残りのマス毎にファイルに分けて出力する。
                    for (n,b) in buf.iter_mut().enumerate() {
                        if b.len() < THREASHOLD_BYTES {continue;}

                        let dir = std::path::Path::new(outdir);
                        if !dir.is_dir() {
                            if let Err(e) =
                                    std::fs::create_dir_all(outdir) {
                                panic!("failed to create dir \"{outdir:?}\" : {e}");
                            }
                        }
                        let mut dest_file = outdir.clone();
                        dest_file.push(format!("mate{n}_{suffix}.txt"));
                        let mut f = OpenOptions::new()
                            .create(true).append(true).open(&dest_file).unwrap();
                        f.write_all(b.as_bytes()).unwrap();
                        b.clear();
                    }
                },
                Err(_e) => {
                    // if e == std::sync::mpsc::RecvTimeoutError::Timeout {}
                },
            }
            
        }
    }

    fn run_spread(&mut self) -> Result<(), std::io::Error> {
        if self.mate < 3 || 60 <= self.mate {
            panic!("self.mate < 3 || 60 <= self.mate");
        }

        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(
                ProgressBar::new(self.kifudir.len() as u64 + 1));
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let show_path = self.verbose;
        if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        let mut outdir = std::env::current_dir().unwrap().clone();
        outdir.push(&self.outdir);
        for d in self.kifudir.iter() {
            let pbchild = if self.show_progressbar {
                let pb = self.multibar.add(ProgressBar::new(7));
                    // load, dedup, extract, dedup, augmentation, dedup, store
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}]{wide_bar}[{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("📔📖📕"));
                pb.set_message("loading kifu...");
                Some(pb)
            } else {
                None
            };
            let mut boards = 
                    data_loader::loadkifu_for_mate(
                        &data_loader::findfiles(&format!("./{d}")),
                        d, self.mate, &mut self.log, show_path);
            if let Some(pb) = &pbchild {
                let path = std::path::Path::new(d);
                let fname = path.components().rev().find_map(|c|
                match c {
                std::path::Component::Normal(os_str) => {Some(os_str)},
                _ => {None},
                }).unwrap_or_default();
                pb.set_message(format!("{}", fname.to_string_lossy()));
                pb.inc(1);
            }  // 1

            data_loader::dedupboards(&mut boards, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 2

            let (tx, rx) = std::sync::mpsc::channel::<String>();
            let outdir = outdir.clone();
            let store_thread = std::thread::spawn(move || {
                Self::store_rfen_thread(rx, &outdir, "spread");
            });

            // ruversiに展開してもらう
            let pbgrandchild = if self.show_progressbar {
                let pb = self.multibar.add(
                ProgressBar::new(boards.len() as u64));
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("🥚🐔🐤"));
                Some(pb)
            } else {
                None
            };
            let mut rr = ruversirunner::RuversiRunner::from_config(
                &std::path::PathBuf::from(
                    self.ruversi_config.clone())).unwrap();
            rr.set_verbose(self.verbose);
            for (ban, _, _, _) in boards.iter() {
                let mates = match rr.run_all_children(&ban.to_string()) {
                    Err(msg) => {panic!("{msg}")},
                    Ok(ban) => {
                        if let Some(pb) = &pbgrandchild {pb.inc(1);}
                        ban
                    },
                };

                let data = mates.join("\n");
                if !data.is_empty() {tx.send(data).unwrap();}
            }
            if let Some(pb) = &pbchild {pb.inc(1);}  // 3

            tx.send(String::new()).unwrap();  // send quit
            store_thread.join().unwrap();
            if let Some(pb) = &pbgrandchild {
                pb.finish();
                self.multibar.remove(pb);
            }
            if let Some(pb) = &pbchild {
                pb.inc(1);  // 7
                pb.finish();
                // self.multibar.remove(pb);
            }
            if let Some(pb ) = &pbtop {pb.inc(1);}
        }

        if let Some(pb ) = &pbtop {
            pb.finish_with_message("done!");
        }
        Ok(())
    }

    fn run_dedup(&mut self) -> Result<(), std::io::Error> {
        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(
                ProgressBar::new(self.kifudir.len() as u64 + 1));
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let show_path = self.verbose;
        if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        let kifudir = self.kifudir.clone();
        for d in kifudir {
            let files = data_loader::findfiles(&format!("./{d}"));
            let pbchild = if self.show_progressbar {
                let pb = self.multibar.add(
                ProgressBar::new(files.len() as u64));
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("📘📖📗"));
                Some(pb)
            } else {
                None
            };
            for path in files {
                if show_path {self.putlog(&path.to_string());}

                // if let Err(e) = self.dedup_rfen(&path, &pbchild) {
                //     panic!("{e} with {path}");
                // }
                if let Err(e) = self.dedup_rfen_in_mem(&path, &pbchild) {
                    panic!("{e} with {path}");
                }

                if let Some(pb) = &pbchild {
                    pb.inc(1);
                }
            }
            if let Some(pb) = &pbchild {
                pb.finish();
            }
            if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        }
        if let Some(pb) = &pbtop {pb.finish();}  // 1
        Ok(())
    }

    /// find txt in a file, 'path'
    fn find_line(txt : &str, path : &str) -> bool {
        if let Ok(fin) = OpenOptions::new().read(true).open(path) {
            let reader = BufReader::new(fin);
            for l in reader.lines() {
                if l.unwrap().starts_with(txt) {
                    return true;
                }
            }
        }

        false
    }

    /// find txt in 'filtered'.
    #[allow(dead_code)]
    fn find_any_in_mem(lines : &[String], filtered : &[String]) -> bool {
        // for l in lines {
        //     // 2分探索が出来るならもっと速く出来る。
        //     if filtered.binary_search(l).is_ok() {
        //         return true;
        //     }
        //     // for l in filtered {
        //     //     if l.starts_with(l2) {
        //     //         return true;
        //     //     }
        //     // }
        // }

        for l in filtered {
            if lines.contains(l) {
                return true;
            }
        }

        false
    }

    /// find any of lines in 'filtered'.
    #[allow(dead_code)]
    fn find_any_in_hashset(lines : &[String],
                           filtered : &std::collections::HashSet<String>)
            -> bool {
        for l in lines {
            let elem = l.split(",").collect::<Vec<_>>();
            if filtered.contains(elem[0]) {
                return true;
            }
        }

        false
    }

    /// find any of lines in a file, 'path'
    #[allow(dead_code)]
    fn find_line_any(lines : &[String], path : &str) -> bool {
        if let Ok(fin) = OpenOptions::new().read(true).open(path) {
            let reader = BufReader::new(fin);
            for l in reader.lines() {
                let l = l.unwrap();
                for l2 in lines {
                    if l.starts_with(l2) {
                        return true;
                    }
                }
            }
        }

        false
    }

    #[allow(dead_code)]
    fn store_mirrored(data : &[String], path : &str) -> Result<(), std::io::Error> {
        let txt = data.join("\n") + "\n";
        let mut fout = OpenOptions::new()
                .create(true).append(true).open(path)?;
        fout.write_all(txt.as_bytes())?;
        fout.flush()?;
        Ok(())
    }

    #[allow(dead_code)]
    fn dedup_rfen(&self, path : &str, pb : &Option<ProgressBar>) -> Result<(), std::io::Error> {
        let path_aug = path.to_string() + ".Aug";
        let path_uniq = path.to_string() + ".Uniq";

        let pathin = std::path::Path::new(path);
        if !pathin.exists() {
            panic!("{path} does not exist!");
        }

        let pbar = {
            let fin = OpenOptions::new().read(true).open(pathin)?;
            let reader = std::io::BufReader::new(fin);
            let lines = reader.lines().count();
            if self.show_progressbar {
                let pb = self.multibar.add(
                    ProgressBar::new(lines as u64));
                    pb.set_style(
                        ProgressStyle::with_template(
                            "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                        .progress_chars("<v>"));
                    Some(pb)
                } else {
                    None
                }
            };

        let fin = OpenOptions::new().read(true).open(pathin)?;
        let reader = std::io::BufReader::new(fin);
        for l in reader.lines() {
            if let Some(pb) = &pbar {pb.inc(1);}
            let line = l.unwrap();
            if line.starts_with("#") {
                continue;  // skip as comment
            }

            let elem = line.split(",").collect::<Vec<&str>>();
            if elem.len() < 2 {panic!("elem.len() < 2 \"{line}\"");}
            // const USE_AUG_FILE : bool = true;  // 重複はaugファイルでやる
            const USE_AUG_FILE : bool = false;  // 重複チェックはメモリでやる
            if USE_AUG_FILE {
                // find a same line in Aug
                if Self::find_line(elem[0], &path_aug) {continue;}

                let score = elem[1];
                let board = match bitboard::BitBoard::try_from(elem[0]) {
                    Ok(b) => {b},
                    Err(e) => {panic!("{e} w/ {line}");},
                };
                let target = board.rotated_mirrored_string(
                        score.parse::<i8>().unwrap());

                Self::store_mirrored(&target[1..], &path_aug)?;
            } else {
                let score = elem[1];
                let board = match bitboard::BitBoard::try_from(elem[0]) {
                    Ok(b) => {b},
                    Err(e) => {panic!("{e} w/ {line}");},
                };
                let target = board.rotated_mirrored_string(
                        score.parse::<i8>().unwrap());

                // find a same line in Aug
                if Self::find_line_any(&target, &path_uniq) {continue;}
            }
            let mut finB = OpenOptions::new()
                    .create(true).append(true).open(&path_uniq)?;
            finB.write_all((line + "\n").as_bytes())?;
        }

        if let Some(pb) = &pbar {
            pb.finish();
            self.multibar.remove(pb);
        }
        Ok(())
    }

    fn dedup_rfen_in_mem(&self, path : &str, pb : &Option<ProgressBar>) -> Result<(), std::io::Error> {
        let path_uniq = path.to_string() + ".Uniq";
        let path_aug = path.to_string() + ".Aug";
        // let mut filtered = Vec::with_capacity(1000000);
        let mut filtered = std::collections::HashSet::new();

        let pathin = std::path::Path::new(path);
        if !pathin.exists() {
            panic!("{path} does not exist!");
        }

        let pbar = {
            let fin = OpenOptions::new().read(true).open(pathin)?;
            let reader = std::io::BufReader::new(fin);
            let lines = reader.lines().count();
            if self.show_progressbar {
                let pb = self.multibar.add(
                    ProgressBar::new(lines as u64));
                    pb.set_style(
                        ProgressStyle::with_template(
                            "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                        .progress_chars("<v>"));
                    Some(pb)
                } else {
                    None
                }
            };

        let (tx, rx) = mpsc::channel::<String>();
        let file_store = std::thread::spawn(move || {
            let mut buf = String::new();
            loop {
                match rx.recv() {
                Err(e) => {panic!("{e}");},
                Ok(recv) => {
                    if recv.is_empty() {
                        if !buf.is_empty() {
                            let mut fout = OpenOptions::new()
                                .create(true).append(true).open(&path_uniq).unwrap();
                            fout.write_all(buf.as_bytes()).unwrap();
                        }
                        eprintln!("file_store DONE.");
                        return;
                    }

                    buf += &recv;
                    buf += "\n";
                    if buf.len() > 1024 * 1024 {  // 1MB
                        let mut fout = OpenOptions::new()
                            .create(true).append(true).open(&path_uniq).unwrap();
                        fout.write_all(buf.as_bytes()).unwrap();

                        buf.clear();
                    }
                },
                }
            }
        });
        let (tx2, rx2) = mpsc::channel::<String>();
        let file_store_aug = std::thread::spawn(move || {
            let mut buf = String::new();
            loop {
                match rx2.recv() {
                Err(e) => {panic!("{e}");},
                Ok(recv) => {
                    if recv.is_empty() {
                        if !buf.is_empty() {
                            let mut fout = OpenOptions::new()
                                .create(true).append(true).open(&path_aug).unwrap();
                            fout.write_all(buf.as_bytes()).unwrap();
                        }
                        eprintln!("file_store_aug DONE.");
                        return;
                    }

                    buf += &recv;
                    buf += "\n";
                    if buf.len() > 1024 * 1024 {  // 1MB
                        let mut fout = OpenOptions::new()
                            .create(true).append(true).open(&path_aug).unwrap();
                        fout.write_all(buf.as_bytes()).unwrap();

                        buf.clear();
                    }
                },
                }
            }
        });

        let fin = OpenOptions::new().read(true).open(pathin)?;
        let reader = std::io::BufReader::new(fin);
        for l in reader.lines() {
            if let Some(pb) = &pbar {pb.inc(1);}
            let line = l.unwrap();
            if line.starts_with("#") {
                tx.send(line.clone()).unwrap();
                continue;  // skip as comment
            }

            let elem = line.split(",").collect::<Vec<&str>>();
            if elem.len() < 2 {panic!("elem.len() < 2 \"{line}\"");}
            let score = elem[1];
            let board = match bitboard::BitBoard::try_from(elem[0]) {
                Ok(b) => {b},
                Err(e) => {panic!("{e} w/ {line}");},
            };
            let target = board.rotated_mirrored_string(
                    score.parse::<i8>().unwrap());
            if !target.contains(&line) {
                panic!("target.contains(&line)");
            }
            let ban = bitboard::BitBoard::try_from(elem[0]).unwrap();
            let mscore = if score.starts_with("-") {
                score[1..].to_string()
            } else {
                if score == "0" {
                    score.to_string()
                } else {
                    String::from("-") + score
                }
            };
            let c = format!("{},{mscore}", ban.flip_all());
            if !target.contains(&c) {
                panic!("target.contains(&{c}) in {target:?}");
            }
            let d = format!("{},{score}", ban.rotate90());
            if !target.contains(&d) {
                panic!("target.contains(&{d}) in {target:?}");
            }
            // find a same line in Aug
            // if Self::find_any_in_mem(&target, &filtered) {
            //     tx.send(line.clone()).unwrap();
            //     continue;
            // }
            if Self::find_any_in_hashset(&target, &filtered) {
                tx2.send(line.clone()).unwrap();
                continue;
            }

            tx.send(line.clone()).unwrap();
            // filtered.push(line.clone());
            // filtered.sort_unstable();
            // if filtered.binary_search(&line).is_err() {
            //     panic!("filtered.binary_search({line})");
            // }
            filtered.insert(elem[0].to_string());
            // filtered.insert(line.clone());
            // if !filtered.contains(&line) {
            //     panic!("if !filtered.contains(&line)");
            // }
        }
        // let mut finB = OpenOptions::new()
        //         .create(true).append(true).open(&path_uniq)?;
        // // finB.write_all((filtered..join("\n") + "\n").as_bytes())?;
        // finB.write_all((filtered.into_iter().collect::<Vec<String>>()
        //         .join("\n") + "\n").as_bytes())?;
        tx.send(String::new()).unwrap();
        tx2.send(String::new()).unwrap();
        file_store.join().unwrap();
        file_store_aug.join().unwrap();

        if let Some(pb) = &pbar {
            pb.finish();
            self.multibar.remove(pb);
        }
        Ok(())
    }

    /// convert rfen to short rfen
    /// 
    /// ex.
    /// "h/h/h/h/h/h/h/h b,-64" -> "zzl b,-64"
    fn run_shorten(&mut self) -> Result<(), std::io::Error> {
        if self.mate < 3 || 60 <= self.mate {
            panic!("self.mate < 3 || 60 <= self.mate");
        }

        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(
                ProgressBar::new(self.kifudir.len() as u64 + 1));
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let show_path = self.verbose;
        let mut outdir = std::env::current_dir().unwrap().clone();
        outdir.push(&self.outdir);
        if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        for d in self.kifudir.iter() {
            let pbchild = if self.show_progressbar {
                let pb = self.multibar.add(ProgressBar::new(4));
                    // load, dedup, extract, dedup, augmentation, dedup, store
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}]{wide_bar}[{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("📜📔📖"));
                pb.set_message("loading kifu...");
                Some(pb)
            } else {
                None
            };
            let files = data_loader::findfiles(&format!("./{d}"));
            if let Some(pb) = &pbchild {pb.set_length(files.len() as u64 + 4);}
            for fname in files {
                let path = format!("{d}/{fname}");
                {
                    let shared = std::sync::Mutex::new(&self.log);
                    let mut l = shared.lock().unwrap();
                    l.write_all(format!("{path}\n").as_bytes()).unwrap();
                }
                if show_path {print!("{path}\r");}
                if let Some(pb) = &pbchild {pb.set_message(fname.to_string());}
                let mut boards = data_loader::load_mates_all(&path).unwrap();

                data_loader::dedupboards(&mut boards, &mut self.log, show_path);
                if let Some(pb) = &pbchild {pb.inc(1);}  // 2

                let (tx, rx) = std::sync::mpsc::channel::<String>();
                let outdir = outdir.clone();
                let store_thread = std::thread::spawn(move || {
                    Self::store_rfen_thread(rx, &outdir, "shorten");
                });

                // convert to short rfen
                let mut data = String::new();
                for (ban, _, _, score) in boards {
                    // if let Some(pb) = &pbgrandchild {pb.inc(1);}
                    data += &format!("{},{score}\n", ban.to_string_short());
                }
                if !data.is_empty() {tx.send(data).unwrap();}

                if let Some(pb) = &pbchild {pb.inc(1);}  // 3

                tx.send(String::new()).unwrap();  // send quit
                store_thread.join().unwrap();
            }
            if let Some(pb) = &pbchild {pb.inc(1);}  // 1

            // if let Some(pb) = &pbgrandchild {
            //     pb.finish();
            //     self.multibar.remove(pb);
            // }
            if let Some(pb) = &pbchild {
                pb.inc(1);  // 4
                pb.finish();
                // self.multibar.remove(pb);
            }
            if let Some(pb ) = &pbtop {pb.inc(1);}
        }

        if let Some(pb ) = &pbtop {
            pb.finish_with_message("done!");
        }
        Ok(())
    }

    /// validate
    fn run_validate(&mut self) -> Result<(), std::io::Error> {
        if self.mate < 3 || 60 <= self.mate {
            panic!("self.mate < 3 || 60 <= self.mate");
        }

        let pbtop = if self.show_progressbar {
            let pb = self.multibar.add(
                ProgressBar::new(self.kifudir.len() as u64 * 2 + 1));
            Some(pb)
        } else {
            None
        };

        // read kifus and extract moves.
        let show_path = self.verbose;
        let mut outdir = std::env::current_dir().unwrap().clone();
        outdir.push(&self.outdir);
        // let outdir = self.outdir.clone();
        if let Some(pb) = &pbtop {pb.inc(1);}  // 1
        for d in self.kifudir.iter() {
            let files = data_loader::findfiles(&format!("./{d}"));
            if let Some(pb) = &pbtop {pb.inc(1);}  // 2n

            let pbchild = if self.show_progressbar {
                let pb = self.multibar.add(ProgressBar::new(files.iter().len() as u64));
                    // load, dedup, extract, dedup, augmentation, dedup, store
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}]{wide_bar}[{eta_precise}] {pos}/{len} {msg}").unwrap()
                    .progress_chars("📜📔📖"));
                pb.set_message("loading kifu...");
                Some(pb)
            } else {
                None
            };
            for fname in files {
                let path = format!("{d}/{fname}");
                {
                    let shared = std::sync::Mutex::new(&self.log);
                    let mut l = shared.lock().unwrap();
                    l.write_all(format!("{path}\n").as_bytes()).unwrap();
                }
                if show_path {print!("{path}\r");}
                if let Some(pb) = &pbchild {pb.set_message(fname.to_string());}
                let mut boards = data_loader::load_mates_all(&path).unwrap();

                data_loader::dedupboards(&mut boards, &mut self.log, show_path);
                if let Some(pb) = &pbchild {pb.inc(1);}  // 2

                let outdir = outdir.clone();
                let (tx, rx) = std::sync::mpsc::channel::<String>();
                let store_thread = std::thread::spawn(move || {
                    Self::store_rfen_thread(rx, &outdir, "validate");
                });

                // validate score w/ ruversi
                let pbgrandchild = if self.show_progressbar {
                    let pb = self.multibar.add(
                    ProgressBar::new(boards.len() as u64));
                    pb.set_style(
                        ProgressStyle::with_template(
                            "[{elapsed_precise}] {wide_bar} [{eta_precise}] {pos}/{len} {msg}").unwrap()
                        .progress_chars("🥚🐔🐤"));
                    Some(pb)
                } else {
                    None
                };
                for (ban, _, _, score) in boards {
                    let data;
                    // if let Some(pb) = &pbgrandchild {pb.inc(1);}
                    // data += &format!("{},{score}\n", ban.to_string_short());
                    // eprintln!("{},{score}", ban.to_string_short());
                    let mut rr = ruversirunner::RuversiRunner::from_config(
                        &std::path::PathBuf::from(
                            self.ruversi_config.clone())).unwrap();
                    rr.set_verbose(self.verbose);
                    match rr.run(&ban.to_string()) {
                        Err(msg) => {panic!("{msg}")},
                        Ok((_txt, new_score)) => {
                            // eprintln!("(,): {txt}, {new_score}");
                            //if let Some(pb) = &pbgrandchild {pb.inc(1);}
                            data = format!("{},{}", ban.to_string_short(),
                                if new_score.parse::<f32>().unwrap() * (score as f32) < 0f32 {
                                    // eprintln!("{new_score} != {score}");
                                    -score
                                } else {
                                    score
                                });
                        },
                    }
                    if !data.is_empty() {tx.send(data).unwrap();}
                    if let Some(pb) = &pbgrandchild {pb.inc(1);}
                }
                if let Some(pb) = &pbchild {pb.inc(1);}  // 3

                tx.send(String::new()).unwrap();  // send quit
                store_thread.join().unwrap();
                if let Some(pb) = &pbgrandchild {
                    pb.finish();
                    self.multibar.remove(pb);
                }
            }
            if let Some(pb) = &pbchild {pb.inc(1);}  // 1

            // if let Some(pb) = &pbgrandchild {
            //     pb.finish();
            //     self.multibar.remove(pb);
            // }
            if let Some(pb) = &pbchild {
                pb.inc(1);  // 4
                pb.finish();
                // self.multibar.remove(pb);
            }
            if let Some(pb ) = &pbtop {pb.inc(1);}
        }

        if let Some(pb ) = &pbtop {
            pb.finish_with_message("done!");
        }
        Ok(())
    }

    fn putlog(&mut self, msg : &str) {
        let msg = if msg.ends_with("\n") {
            msg
        } else {
            &(msg.to_string() + "\n")
        };
        self.log.write_all(msg.as_bytes()).unwrap();
        self.log.sync_all().unwrap();
        if !self.show_progressbar {
            print!("{msg}");
            std::io::stdout().flush().unwrap();
        }
    }
}
