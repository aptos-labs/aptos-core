use move_command_line_common::files::FileHash;
use movefmt::{
    core::token_tree::{CommentExtrator, CommentExtratorErr, TokenTree},
    tools::movefmt_diff,
    tools::syntax::parse_file_string,
    tools::utils::*,
};
use std::collections::BTreeSet;
use tracing_subscriber::EnvFilter;

use move_compiler::{parser::lexer::Lexer, shared::CompilationEnv, Flags};
use std::path::Path;

fn scan_dir(dir: &str) -> usize {
    let mut num: usize = 0;
    for x in walkdir::WalkDir::new(dir) {
        let x = match x {
            Ok(x) => x,
            Err(_) => {
                return num;
            }
        };
        if x.file_type().is_file()
            && x.file_name().to_str().unwrap().ends_with(".move")
            && !x.file_name().to_str().unwrap().contains(".fmt")
            && !x.file_name().to_str().unwrap().contains(".out")
        {
            let p = x.clone().into_path();
            test_on_file(p.as_path());
            num += 1;

            let index = p.to_str().unwrap().rfind(".").unwrap();
            let mut expected_filename = p.to_str().unwrap()[0..index].to_string();
            expected_filename.push_str(".fmt.move");

            let mut actual_filename = p.to_str().unwrap()[0..index].to_string();
            actual_filename.push_str(".fmt.out");

            movefmt_diff::assert_output(Path::new(&actual_filename), Path::new(&expected_filename));
        }
    }
    num
}

#[test]
fn test_single_file() {
    eprintln!("================== test_single_file ===================");
    std::env::set_var("MOVEFMT_LOG", "movefmt=DEBUG");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("MOVEFMT_LOG"))
        .init();

    test_on_file(&Path::new("./tests/complex/input4.move"));
}

fn test_on_file(p: impl AsRef<Path>) {
    let p = p.as_ref();
    eprintln!("try format:{:?}", p);
    let content_origin = std::fs::read_to_string(&p).unwrap();
    {
        let attrs: BTreeSet<String> = BTreeSet::new();
        let mut env = CompilationEnv::new(Flags::testing(), attrs);
        match parse_file_string(&mut env, FileHash::empty(), &content_origin) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("file '{:?}' skipped because of parse not ok", p);
                return;
            }
        }
    }
    let content_origin = std::fs::read_to_string(p).unwrap();
    test_content(content_origin.as_str(), p);
}

fn test_content(content_origin: &str, p: impl AsRef<Path>) {
    let p = p.as_ref();
    let tokens_origin =
        extract_tokens(content_origin).expect("test file should be about to lexer,err:{:?}");

    let content_format =
        movefmt::core::fmt::format_entry(content_origin, commentfmt::Config::default()).unwrap();

    let tokens_format = match extract_tokens(content_format.as_str()) {
        Ok(x) => x,
        Err(err) => {
            unreachable!(
                "should be able to parse after format:err{:?},after format:\n\n################\n{}\n###############",
                err,
                content_format
            );
        }
    };
    for (t1, t2) in tokens_origin.iter().zip(tokens_format.iter()) {
        assert_eq!(
            t1.content,
            t2.content,
            "format not ok,file:{:?} line:{} col:{},after format line:{} col:{}",
            p,
            // +1 in vscode UI line and col start with 1
            t1.line + 1,
            t1.col + 1,
            t2.line + 1,
            t2.col + 1,
        );
    }
    assert_eq!(
        tokens_origin.len(),
        tokens_format.len(),
        "{:?} tokens count should equal",
        p
    );
    let comments_origin = extract_comments(&content_origin).unwrap();
    let comments_format = extract_comments(&content_format).unwrap();
    for (index, (c1, c2)) in comments_origin
        .iter()
        .zip(comments_format.iter())
        .enumerate()
    {
        assert_eq!(c1, c2, "comment {} not ok.", index);
    }
    assert_eq!(
        comments_origin.len(),
        comments_format.len(),
        "{:?} comments count should equal",
        p,
    );

    let result_file_path = mk_result_filepath(&p.to_path_buf());
    let _ = std::fs::write(result_file_path.clone(), content_format);
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ExtractToken {
    content: String,
    line: u32,
    col: u32,
}

fn extract_comments(content: &str) -> Result<Vec<String>, CommentExtratorErr> {
    let c = CommentExtrator::new(content)?;
    let c: Vec<_> = c
        .comments
        .into_iter()
        .map(|x| x.content)
        .map(|x| x.replacen(" ", "", usize::MAX))
        .map(|x| x.replacen("\t", "", usize::MAX))
        .map(|x| x.replacen("\n", "", usize::MAX))
        .collect();
    return Ok(c);
}

fn extract_tokens(content: &str) -> Result<Vec<ExtractToken>, Vec<String>> {
    let p = Path::new(".").to_path_buf();
    let mut line_mapping = FileLineMapping::default();
    line_mapping.update(p.clone(), &content);
    let filehash = FileHash::empty();
    let attrs: BTreeSet<String> = BTreeSet::new();
    let mut env = CompilationEnv::new(Flags::testing(), attrs);
    let (defs, _comments) = match parse_file_string(&mut env, filehash, content) {
        Ok(x) => x,
        Err(d) => {
            let mut ret = Vec::with_capacity(d.len());
            for x in d.into_codespan_format() {
                let (_s, msg, (loc, m), _, _notes) = x;
                let loc = line_mapping.translate(&p, loc.start(), loc.end()).unwrap();
                ret.push(format!(
                    "{}:{} {}",
                    loc.line_start + 1,
                    loc.col_start + 1,
                    format!("{}\n{}", msg, m)
                ));
            }
            return Err(ret);
        }
    };
    let lexer = Lexer::new(&content, filehash);
    let mut ret = Vec::new();
    let parse = movefmt::core::token_tree::Parser::new(lexer, &defs);
    let token_tree = parse.parse_tokens();
    let mut line_mapping = FileLineMapping::default();
    line_mapping.update(p.to_path_buf(), &content);
    fn collect_token_tree(ret: &mut Vec<ExtractToken>, m: &FileLineMapping, t: &TokenTree) {
        match t {
            TokenTree::SimpleToken {
                content,
                pos,
                tok: _tok,
                note: _,
            } => {
                let loc = m
                    .translate(&Path::new(".").to_path_buf(), *pos, *pos)
                    .unwrap();

                ret.push(ExtractToken {
                    content: content.clone(),
                    line: loc.line_start,
                    col: loc.col_start,
                });
            }
            TokenTree::Nested {
                elements,
                kind,
                note: _,
            } => {
                let start_loc = m
                    .translate(
                        &Path::new(".").to_path_buf(),
                        kind.start_pos,
                        kind.start_pos,
                    )
                    .unwrap();
                ret.push(ExtractToken {
                    content: format!("{}", kind.kind.start_tok()),
                    line: start_loc.line_start,
                    col: start_loc.col_start,
                });

                for token in elements.iter() {
                    collect_token_tree(ret, m, token);
                }
                let end_loc = m
                    .translate(&Path::new(".").to_path_buf(), kind.end_pos, kind.end_pos)
                    .unwrap();
                ret.push(ExtractToken {
                    content: format!("{}", kind.kind.end_tok()),
                    line: end_loc.line_start,
                    col: end_loc.col_start,
                });
            }
        }
    }
    for token in token_tree.iter() {
        collect_token_tree(&mut ret, &line_mapping, token);
    }

    Ok(ret)
}

#[test]
fn test_success_dir() {
    std::env::set_var("MOVEFMT_LOG", "movefmt=DEBUG");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("MOVEFMT_LOG"))
        .init();

    let mut fmt_num = 0;
    // eprintln!("formated {} files", scan_dir("./tests/formatter/use"));
    fmt_num = fmt_num + scan_dir("./tests/formatter/tuple");
    fmt_num = fmt_num + scan_dir("./tests/formatter/expr");
    fmt_num = fmt_num + scan_dir("./tests/formatter/fun");
    fmt_num = fmt_num + scan_dir("./tests/formatter/struct");
    fmt_num = fmt_num + scan_dir("./tests/formatter/list");
    fmt_num = fmt_num + scan_dir("./tests/formatter/spec_struct");
    fmt_num = fmt_num + scan_dir("./tests/formatter/spec_module");
    fmt_num = fmt_num + scan_dir("./tests/formatter/lambda");
    fmt_num = fmt_num + scan_dir("./tests/formatter/spec_fun");
    fmt_num = fmt_num + scan_dir("./tests/formatter/other");
    eprintln!("formated {} files", fmt_num);
}

#[test]
fn test_dir() {
    std::env::set_var("MOVEFMT_LOG", "movefmt=DEBUG");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("MOVEFMT_LOG"))
        .init();
    eprintln!("formated {} files", scan_dir("./tests/complex/branch"));
}

#[test]
fn regression_test_main() {
    let mut num: usize = 0;
    for ten_dir in walkdir::WalkDir::new("./tests/formatter") {
        let ten_dir = match ten_dir {
            Ok(ten_dir) => ten_dir,
            Err(_) => {
                eprintln!("formated {} files", num);
                return;
            }
        };
        if !ten_dir.file_type().is_dir() {
            eprintln!("formated {} files", num);
            return;
        }
        eprintln!("cur_dir = {:?}", ten_dir.file_name().to_str());
        num = num + scan_dir(ten_dir.path().to_str().unwrap());
    }
    eprintln!("formated {} files", num);
}
