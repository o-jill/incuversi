use core::fmt;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::fmt::{Display, Formatter};
use std::process::{Child, Command, Stdio};

/**
 * configファイルのargsタグを処理する。
 * # Argumemts  
 * - txt args:を含まない"arg:"の後ろに書いてある文字列。
 * # Returns  
 * - Ok(Vec<String>)  
 *   引数に渡したい文字列の配列
 * - Err(String>)  
 *   処理エラーの内容
 */
fn parse_args_tag(txt : &str) -> Result<Vec<String>, String> {
    let args = txt.trim().split(",")
        .map(|s| s.trim().to_string()).collect::<Vec<_>>();
    if args.len() > 1 || !args[0].is_empty() {
        for (i, a) in args.iter().enumerate() {
            if a.is_empty() {
                return Err(
                    format!("\"{txt}\" contains empty part @{i}! {args:?}"));
            }
        }
        Ok(args)
    } else  {
        Ok(Vec::new())
    }
}

// const OBF : &str = "test.obf";
const CD : &str = "../../edax-reversi/";
const EXE_PATH : &str = "./bin/lEdax-x64-modern";
const EVFILE : &str = "data/eval.dat";

pub struct CassioRunner {
    curdir : String,
    path : String,
    evfile : String,
    cas : String,
    args : Vec<String>,
    // verbose : bool,
}

impl Display for CassioRunner {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "curdir:{}, path:{}, evfile:{}, cassio:{}, args:{:?}",
                self.curdir, self.path, self.evfile, self.cas, self.args)
    }
}

impl CassioRunner {
    pub fn new() -> Self {
        Self {
            curdir: String::from(CD),
            path: String::from(EXE_PATH),
            evfile: String::from(EVFILE),
            cas: String::from("-cassio"),
            args: Vec::new(),
        }
    }

    // pub fn set_verbose(&mut self, verbose : bool) {
    //     self.verbose = verbose;
    // }

    pub fn from_config(path : &std::path::PathBuf)
            -> Result<CassioRunner, String> {
        let mut rr = CassioRunner::new();
        if path.as_os_str().is_empty() {
            return Ok(rr);
        }

        match rr.read(path) {
            Ok(_) => Ok(rr),
            Err(msg) => Err(msg),
        }
    }

    /// read config from a file.
    /// 
    /// ex.
    /// curdir: ~/ruversi/
    /// path: ./bin/ruversi
    /// evfile: ./data/eval.dat
    /// args: --some,arguments,if,needed
    /// 
    /// note:
    /// COMMAs between `"`s are not be ignored yet...
    /// "a,b" will be `"a` and `b"`...
    pub fn read(&mut self, path : &std::path::PathBuf) -> Result<(), String> {
        let file = File::open(path);
        if file.is_err() {return Err(file.err().unwrap().to_string());}

        let file = file.unwrap();
        let lines = BufReader::new(file);
        for line in lines.lines() {
            match line {
                Ok(l) => {
                    if let Some(cd) = l.strip_prefix("curdir:") {
                        // println!("{l}");
                        self.curdir = String::from(cd.trim());
                    } else if let Some(ed) = l.strip_prefix("path:") {
                        // println!("{l}");
                        self.path = String::from(ed.trim());
                    } else if let Some(ed) = l.strip_prefix("edax:") {
                        // println!("{l}");
                        self.path = String::from(ed.trim());
                    } else if let Some(evf) = l.strip_prefix("evfile:") {
                        // println!("{l}");
                        self.evfile = String::from(evf.trim());
                    } else if let Some(cas) = l.strip_prefix("cas:") {
                        // println!("{l}");
                        self.cas = String::from(cas.trim());
                    } else if let Some(args_txt) = l.strip_prefix("args:") {
                        self.args = parse_args_tag(args_txt)?;
                    }
                },
                Err(err) => {return Err(err.to_string())}
            }
        }
        Ok(())
    }

    fn spawn(&self) -> std::io::Result<Child> {
// println!("args:{:?}", self.args);
        // std::env::set_current_dir(&self.curdir).unwrap();
        let mut cmd = Command::new(&self.path);
        cmd.current_dir(&self.curdir)
            .arg(&self.cas)
            // .arg("-eval-file").arg(&self.evfile).args(&self.args)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            // .stderr(Stdio::null());
        // println!("cmd:{cmd:?}");
        let ret = cmd.spawn();
        if let Err(e) = ret {
            panic!("spawn error: {e}");
        }
        ret
    }

    pub fn run(&self) -> Result<Child, String> {
        // launch cassio
        // let curdir = std::env::current_dir().unwrap();
        match self.spawn() {
            Err(msg) => {
                // std::env::set_current_dir(curdir).unwrap();
                Err(format!("error running cassio... [{msg}], {self}"))
            },
            Ok(prcs) => {
                // std::env::set_current_dir(curdir).unwrap();
                Ok(prcs)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::fs;

    #[test]
    fn test_cassiorunner_default_values() {
        // CassioRunner::new() で各フィールドがデフォルト値になっていることを確認
        let cr = CassioRunner::new();
        assert_eq!(cr.curdir, "../../edax-reversi/");
        assert_eq!(cr.path, "./bin/lEdax-x64-modern");
        assert_eq!(cr.evfile, "data/eval.dat");
        assert_eq!(cr.cas, "-cassio");
    }

    #[test]
    fn test_cassiorunner_to_str() {
        // to_str の内容がフィールドに基づくことを確認
        let cr = CassioRunner::new();
        let s = cr.to_string();
        assert!(s.contains("curdir:"));
        assert!(s.contains("path:"));
        assert!(s.contains("evfile:"));
        assert!(s.contains("cassio:"));
    }

    #[test]
    fn test_cassiorunner_read_config_file() {
        // 設定ファイルを作成し、read で値が読み込まれることを確認
        let tmp = std::env::temp_dir();
        let config_path =
                tmp.join("test_cassiorunner_config.txt");
        let contents = "\
curdir:/tmp/mycassio
path:/tmp/mycassio_path
evfile:/tmp/mycassio_evfile.txt
cas:--cassioX
";
        {
            let cp = config_path.clone();
            let mut file = File::create(cp).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
        }
        let mut cr = CassioRunner::new();
        let result = cr.read(&config_path);
        assert!(result.is_ok());
        assert_eq!(cr.curdir, "/tmp/mycassio");
        assert_eq!(cr.path, "/tmp/mycassio_path");
        assert_eq!(cr.evfile, "/tmp/mycassio_evfile.txt");
        assert_eq!(cr.cas, "--cassioX");
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_cassiorunner_read_config_file_empty_arg1() {
        // 一時ファイルに設定を書き込み、
        // read で値が読み込まれ、
        // 空文字の引数が含まれているErrが返る事を確認
        let tmp = std::env::temp_dir();
        let config_path =
                tmp.join("test_cassiorunner_config_arg1.txt");
        let contents = "\
curdir:/tmp/mycassio
path:/tmp/mycassio_path
evfile:/tmp/mycassio_evfile.txt
cas:--cassioX
args:a,b,c,
";

        // 前のテストのファイルが残ってたら消す
        if config_path.exists() {
            println!("removed config file for test.");
            fs::remove_file(&config_path).unwrap();
        }

        {
            let cp = config_path.clone();
            let mut file = File::create(cp).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
        }
        let mut rr = CassioRunner::new();
        let result = rr.read(&config_path);
        assert_eq!(result, Err("\"a,b,c,\" contains empty part @3! [\"a\", \"b\", \"c\", \"\"]".to_string()));
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_cassiorunner_read_config_file_empty_arg2() {
        // 一時ファイルに設定を書き込み、
        // read で値が読み込まれ、
        // 空文字の引数が含まれているErrが返る事を確認
        let tmp = std::env::temp_dir();
        let config_path =
                tmp.join("test_cassiorunner_config_arg2.txt");
        let contents = "\
curdir:/tmp/mycassio
path:/tmp/mycassio_path
evfile:/tmp/mycassio_evfile.txt
cas:--cassioX
args:a,b,,c
";

        // 前のテストのファイルが残ってたら消す
        if config_path.exists() {
            println!("removed config file for test.");
            fs::remove_file(&config_path).unwrap();
        }

        {
            let cp = config_path.clone();
            let mut file = File::create(cp).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
        }
        let mut cr = CassioRunner::new();
        let result = cr.read(&config_path);
        assert_eq!(result, Err("\"a,b,,c\" contains empty part @2! [\"a\", \"b\", \"\", \"c\"]".to_string()));
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_cassiorunner_read_config_file_empty_arg3() {
        // 一時ファイルに設定を書き込み、
        // read で値が読み込まれることを確認
        // args:の後ろが空白だけでもエラーに鳴らないことの確認
        let tmp = std::env::temp_dir();
        let config_path =
                tmp.join("test_cassiorunner_config_arg3.txt");
        let contents = "\
curdir:/tmp/mycassio
path:/tmp/mycassio_path
evfile:/tmp/mycassio_evfile.txt
cas:--cassioX
args:  
";

        // 前のテストのファイルが残ってたら消す
        if config_path.exists() {
            println!("removed config file for test.");
            fs::remove_file(&config_path).unwrap();
        }

        {
            let cp = config_path.clone();
            let mut file = File::create(cp).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
        }
        let mut cr = CassioRunner::new();
        let result = cr.read(&config_path);
        assert_eq!(result, Ok(()));
        assert_eq!(cr.curdir, "/tmp/mycassio");
        assert_eq!(cr.path, "/tmp/mycassio_path");
        assert_eq!(cr.evfile, "/tmp/mycassio_evfile.txt");
        assert_eq!(cr.cas, "--cassioX");
        assert!(cr.args.is_empty());
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_cassiorunner_read_config_file_empty_arg4() {
        // 一時ファイルに設定を書き込み、
        // read で値が読み込まれることを確認
        // args:の後ろが空でもエラーに鳴らないことの確認
        let tmp = std::env::temp_dir();
        let config_path =
                tmp.join("test_cassiorunner_config_arg4.txt");
        let contents = "\
curdir:/tmp/mycassio
path:/tmp/mycassio_path
evfile:/tmp/mycassio_evfile.txt
cas:--cassioX
args:
";

        // 前のテストのファイルが残ってたら消す
        if config_path.exists() {
            println!("removed config file for test.");
            fs::remove_file(&config_path).unwrap();
        }

        {
            let cp = config_path.clone();
            let mut file = File::create(cp).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
        }
        let mut cr = CassioRunner::new();
        let result = cr.read(&config_path);
        assert_eq!(result, Ok(()));
        assert_eq!(cr.curdir, "/tmp/mycassio");
        assert_eq!(cr.path, "/tmp/mycassio_path");
        assert_eq!(cr.evfile, "/tmp/mycassio_evfile.txt");
        assert_eq!(cr.cas, "--cassioX");
        assert!(cr.args.is_empty());
        fs::remove_file(config_path).unwrap();
    }

    #[test]
    fn test_cassiorunner_read_config_file_not_found() {
        // 存在しないファイルを指定した場合、Errが返ることを確認
        let mut cr = CassioRunner::new();
        let result = cr.read(
            &std::path::PathBuf::from("/tmp/no_such_cassio_config.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cassiorunner_from_config_empty() {
        // from_config("") でデフォルト値
        let cr = CassioRunner::from_config(
            &std::path::PathBuf::from("")).unwrap();
        assert_eq!(cr.curdir, "../../edax-reversi/");
    }

    #[test]
    fn test_cassiorunner_from_config_file() {
        // from_config で設定ファイルを読み取る
        let tmp = std::env::temp_dir();
        let config_path =
                tmp.join("test_cassiorunner2_config.txt");
        let contents = "cas:--abc\n";
        {
            let cp = config_path.clone();
            let mut file = File::create(cp).unwrap();
            file.write_all(contents.as_bytes()).unwrap();
        }
        let cr = CassioRunner::from_config(&config_path).unwrap();
        assert_eq!(cr.cas, "--abc");
        fs::remove_file(config_path).unwrap();
    }
}
