//! The site's own stylesheets and scripts, minified as they are copied in. The sources
//! under `web/` stay readable; the published copies carry no comments and no indentation.
//! A stylesheet is reduced here by a small pass that knows strings and comments, since the
//! stylesheets are the site's own and use nothing the pass cannot carry through untouched.
//! A script goes through oxc, a real parser, so a construct it does not understand fails
//! the build rather than reaching a browser.

use std::fs;
use std::path::Path;

use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenOptions};
use oxc::minifier::{Minifier, MinifierOptions};
use oxc::parser::Parser;
use oxc::span::SourceType;

/// Minify every stylesheet and script under `dir` in place: a `.css` file by [`css`], a
/// `.js` file by [`js`] as a module, since the dashboard's scripts all are. A file already
/// named `.min.js` is left as it is, and so is everything else.
///
/// # Arguments
///
/// * `dir` - the directory, walked recursively.
///
/// # Returns
///
/// The files rewritten, as (path, bytes before, bytes after).
///
/// # Errors
///
/// When a file cannot be read or written, or a script does not parse.
pub fn directory(dir: &Path) -> Result<Vec<(String, usize, usize)>, String> {
    let mut done = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries =
            fs::read_dir(&next).map_err(|err| format!("reading {}: {err}", next.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|err| format!("reading {}: {err}", next.display()))?
                .path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let name = path.to_string_lossy().replace('\\', "/");
            let minified = if name.ends_with(".css") {
                Some(css(&read(&path)?))
            } else if name.ends_with(".js") && !name.ends_with(".min.js") {
                Some(js(&read(&path)?, true).map_err(|err| format!("{name}: {err}"))?)
            } else {
                None
            };
            if let Some(body) = minified {
                let before = fs::metadata(&path)
                    .map(|meta| usize::try_from(meta.len()).unwrap_or(usize::MAX))
                    .unwrap_or(0);
                fs::write(&path, &body).map_err(|err| format!("writing {name}: {err}"))?;
                let shown = name
                    .strip_prefix(&dir.to_string_lossy().replace('\\', "/"))
                    .unwrap_or(&name)
                    .trim_start_matches('/')
                    .to_owned();
                done.push((shown, before, body.len()));
            }
        }
    }
    done.sort();
    Ok(done)
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("reading {}: {err}", path.display()))
}

/// Minify a script: parse it, compress and mangle it, and print it without whitespace.
///
/// # Arguments
///
/// * `source` - the script's text.
/// * `module` - whether it is an ES module, with imports or exports, rather than a
///   classic script.
///
/// # Returns
///
/// The minified text.
///
/// # Errors
///
/// When the script does not parse.
pub fn js(source: &str, module: bool) -> Result<String, String> {
    let allocator = Allocator::default();
    let source_type = if module {
        SourceType::mjs()
    } else {
        SourceType::cjs()
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if let Some(error) = parsed.diagnostics.first() {
        return Err(format!("{error}"));
    }
    let mut program = parsed.program;
    let minified = Minifier::new(MinifierOptions::default()).minify(&allocator, &mut program);
    Ok(Codegen::new()
        .with_options(CodegenOptions::minify())
        .with_scoping(minified.scoping)
        .build(&program)
        .code)
}

/// Minify a stylesheet: comments out, and whitespace down to what the grammar needs.
/// Strings are carried through untouched, a space before a colon stays (it separates a
/// descendant from its pseudo-class), and the spaces inside `calc()` and the like stay,
/// since only the space around braces, semicolons, commas, colons, and child combinators
/// is dropped.
///
/// # Arguments
///
/// * `source` - the stylesheet's text.
///
/// # Returns
///
/// The minified text.
pub fn css(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut pending = false;
    let mut swallow = true;
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = '\0';
                for d in chars.by_ref() {
                    if previous == '*' && d == '/' {
                        break;
                    }
                    previous = d;
                }
                pending = true;
            }
            c if c.is_whitespace() => pending = true,
            '{' | '}' | ';' | ',' | '>' => {
                if c == '}' && out.ends_with(';') {
                    out.pop();
                }
                out.push(c);
                pending = false;
                swallow = true;
            }
            ':' => {
                if pending && !swallow {
                    out.push(' ');
                }
                out.push(':');
                pending = false;
                swallow = true;
            }
            _ => {
                if pending && !swallow {
                    out.push(' ');
                }
                pending = false;
                swallow = false;
                out.push(c);
                if c == '"' || c == '\'' {
                    let mut escaped = false;
                    for d in chars.by_ref() {
                        out.push(d);
                        if escaped {
                            escaped = false;
                        } else if d == '\\' {
                            escaped = true;
                        } else if d == c {
                            break;
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stylesheet_loses_its_comments_and_its_whitespace_but_not_its_meaning() {
        let source = "/* the header */\n.a > .b , .c:hover {\n  color: var(--x) ;\n  margin: 0 1px ;\n}\n\n@media (max-width: 860px) { .a { width: calc(100% - 2rem) } }\n.x :hover { }\n.a::before { content: \" > ; \" ; }\n@font-face { src: url(\"/fonts/x.woff2\") format(\"woff2\"); }\n";
        assert_eq!(
            css(source),
            ".a>.b,.c:hover{color:var(--x);margin:0 1px}@media (max-width:860px){.a{width:calc(100% - 2rem)}}.x :hover{}.a::before{content:\" > ; \"}@font-face{src:url(\"/fonts/x.woff2\") format(\"woff2\")}"
        );
        assert_eq!(css("a{b:'it\\'s'}"), "a{b:'it\\'s'}");
    }

    #[test]
    fn a_script_is_compressed_and_a_module_keeps_its_exports() {
        let script = js(
            "// a comment\nconst answer = 1 + 1;\nconsole.log(answer);\n",
            false,
        )
        .unwrap();
        assert!(script.contains("console.log"), "{script}");
        assert!(
            !script.contains("comment") && !script.contains('\n'),
            "{script}"
        );
        let module = js(
            "import { mount } from './consoles.js';\nexport function init() { mount(); return 1 + 1; }\n",
            true,
        )
        .unwrap();
        assert!(
            module.contains("export") && module.contains("init"),
            "{module}"
        );
        assert!(module.contains("./consoles.js"), "{module}");
    }

    #[test]
    fn a_script_that_does_not_parse_fails_the_build() {
        assert!(js("const = ;", false).is_err());
    }

    #[test]
    fn a_directory_is_minified_in_place_and_the_rest_is_left_alone() {
        let dir = std::env::temp_dir().join(format!("pamoja-minify-{}", std::process::id()));
        fs::create_dir_all(dir.join("app")).unwrap();
        fs::write(dir.join("global.css"), "/* c */\n.a {\n  color: red;\n}\n").unwrap();
        fs::write(
            dir.join("app/app.js"),
            "import { x } from './x.js';\n// c\nexport const y = x + 1;\n",
        )
        .unwrap();
        fs::write(dir.join("zquery.min.js"), "const  a=1;\n").unwrap();
        fs::write(dir.join("state.json"), "{ \"a\": 1 }\n").unwrap();
        let done = directory(&dir).unwrap();
        assert_eq!(
            done.iter().map(|(p, _, _)| p.as_str()).collect::<Vec<_>>(),
            ["app/app.js", "global.css"]
        );
        assert_eq!(
            fs::read_to_string(dir.join("global.css")).unwrap(),
            ".a{color:red}"
        );
        let app = fs::read_to_string(dir.join("app/app.js")).unwrap();
        assert!(
            app.contains("export") && app.contains("./x.js") && !app.contains("// c"),
            "{app}"
        );
        assert_eq!(
            fs::read_to_string(dir.join("zquery.min.js")).unwrap(),
            "const  a=1;\n"
        );
        assert_eq!(
            fs::read_to_string(dir.join("state.json")).unwrap(),
            "{ \"a\": 1 }\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
