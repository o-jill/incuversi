use super::*;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};


pub struct Incubator {
    kifudir : Vec<String>,
    log : std::fs::File,
    mate : u32,
    // matefiles : String,
    mode : argument::Mode,
    multibar : MultiProgress,
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
        Err(e) => {panic!("{e}")},
        };

        let mode = arg.md;
        let kifudir = arg.kifudir;
        // let matefiles = arg.mate_file.unwrap_or(String::new()).clone();
        let ruversi_config = arg.ru_config.unwrap_or(String::new());
        let mate = arg.mate;
        let verbose = arg.verbose;

        Self {
            kifudir,
            log,
            mate,
            // matefiles,
            mode,
            multibar : MultiProgress::new(),
            ruversi_config,
            show_progressbar : arg.progressbar,
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

            // augmentation
            let mut newmates = mates.iter().flat_map(|(ban, fsb, fsw, score)| {
                ban.rotated_mirrored(*fsb, *fsw, *score)
            }).collect::<Vec<_>>();
                if let Some(pb) = &pbchild {pb.inc(1);}  // 5

            data_loader::dedupboards(&mut newmates, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 6
            if newmates.is_empty() {panic!("mates: {}", newmates.len());}

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
                let mut f = std::fs::OpenOptions::new()
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
            let mut boards =
                data_loader::findfiles(&format!("./{d}")).iter().flat_map(
                    |fname|
                    {
                        let path = format!("{d}/{fname}");
                        data_loader::load_mates(&path, self.mate).unwrap()
                    }
                    ).collect();
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
            let rr = ruversirunner::RuversiRunner::from_config(
                &std::path::PathBuf::from(
                    self.ruversi_config.clone())).unwrap();
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
            let mut newmates = mates.iter().flat_map(|(ban, fsb, fsw, score)| {
                ban.rotated_mirrored(*fsb, *fsw, *score)
            }).collect::<Vec<_>>();
                if let Some(pb) = &pbchild {pb.inc(1);}  // 5

            data_loader::dedupboards(&mut newmates, &mut self.log, show_path);
            if let Some(pb) = &pbchild {pb.inc(1);}  // 6
            if newmates.is_empty() {panic!("mates: {}", newmates.len());}

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
                let mut f = std::fs::OpenOptions::new()
                    .create(true).append(true).open(&dest_file).unwrap();
                f.write_all(text.as_bytes()).unwrap();
            }

            if let Some(pb) = &pbchild {
                pb.inc(1);  // 7
                pb.finish();
                self.multibar.remove(pb);
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
            _ => {
                Ok(())
            }
        }
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
