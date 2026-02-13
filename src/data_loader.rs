use super::*;

use rayon::prelude::*;

// const INPUTSIZE :i64 = weight::N_INPUT as i64;

// list up kifu
pub fn findfiles(kifupath : &str) -> Vec<String> {
    // let sta = std::time::Instant::now();
    let dir = std::fs::read_dir(kifupath).unwrap();
    let mut files = dir.filter_map(|entry| {
        entry.ok().and_then(|e|
            e.path().file_name().map(|n|
                n.to_str().unwrap().to_string()
            )
        )}).filter(|fnm| {
            // fnm.contains("kifu")
            fnm.contains(".txt")
        }).collect::<Vec<String>>();
    // println!("{:?}", files);

    files.sort();
    // println!("{}usec",sta.elapsed().as_micros());
    files
}

pub fn loadkifu_for_mate(files : &[String], d : &str, mate : u32,
        log : &mut std::fs::File, show_path : bool)
        -> Vec<(bitboard::BitBoard, i8, i8, i8)> {
    // let sta = std::time::Instant::now();
    let shared = std::sync::Mutex::new(log);
    let boards = files.par_iter().flat_map(|fname| {
        let path = format!("{d}/{fname}");
        {
            let mut l = shared.lock().unwrap();
            l.write_all(format!("{path}\n").as_bytes()).unwrap();
            if show_path {print!("{path}\r");}
        }
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.split('\n').collect();
        let kifu = kifu::Kifu::from(&lines);
        kifu.list.iter().filter_map(|t| {
            let ban = bitboard::BitBoard::from(&t.rfen).unwrap();
            // 指定の局面じゃない
            if ban.is_last_n(mate) {
                let score = ban.count();
                let (fsb, fsw) = ban.fixedstones();
                Some((ban, fsb, fsw, score))
            } else {
                None
            }
        }).collect::<Vec<_>>()
    }).collect();
    if show_path {println!();}
    // println!("{}usec",sta.elapsed().as_micros());
    boards
}

#[allow(dead_code)]
fn read_mate_file(buf : impl std::io::BufRead, mate : u32)
        -> Result<Vec<(bitboard::BitBoard, i8, i8, i8)>, String> {
    let mut ret = Vec::new();

    for line in buf.lines() {
        match line {
            Err(e) => {return Err(format!("{e}"))},
            Ok(l) => {
                // コメント行 or 11文字未満
                if l.len() < 11 || l.starts_with("#") {continue;}
                // rfen,score
                let elem : Vec<&str> = l.split(",").collect();
                let ban = bitboard::BitBoard::from(elem[0])?;
                if !ban.is_last_n(mate) {continue;}

                let (b, w) = ban.fixedstones();
                let score = match elem[1].parse::<i8>() {
                    Err(msg) => {return Err(format!("error: parse score : {msg}"));},
                    Ok(num) => {num},
                };
                ret.push((ban, b, w, score));
            }
        }
    }

    Ok(ret)
}

#[allow(dead_code)]
pub fn load_mates(path : &str, mate : u32)
        -> Result<Vec<(bitboard::BitBoard, i8, i8, i8)>, String> {
    let filepath = std::path::Path::new(path);
    if !filepath.exists() {return Err(format!("{path} does NOT exist!"));}

    if path.ends_with(".zst") || path.ends_with(".zstd") {
        let f = std::fs::File::open(path)
                .map_err(|e| format!("error: {e} @ File::open"))?;
        let z = zstd::Decoder::new(f)
                .map_err(|e| format!("error: {e} @ zstd::Decoder::new"))?;

        let buf = std::io::BufReader::new(z);
        let ret = read_mate_file(buf, mate)?;
        Ok(ret)
    } else {
        let f = std::fs::File::open(path).map_err(|e| format!("{e}"))?;

        let buf = std::io::BufReader::new(f);
        let ret = read_mate_file(buf, mate)?;
        Ok(ret)
    }
}

pub fn dedupboards(boards : &mut Vec<(bitboard::BitBoard, i8, i8, i8)>,
                   log : &mut std::fs::File, show_path : bool) {
    // println!("board: {} boards", boards.len());
    // let sta = std::time::Instant::now();
    boards.sort_by(|a, b| {
        a.0.partial_cmp(&b.0).unwrap()
    });
    boards.dedup_by(|a, b| {a == b});
    // println!("{}usec",sta.elapsed().as_micros());
    let msg = format!("board: {} boards\n", boards.len());
    log.write_all(msg.as_bytes()).unwrap();
    if show_path {print!("{msg}");}
}
