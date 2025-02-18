use failure::Fallible;
use rga::adapters::spawning::map_exe_error;
use rga::adapters::*;
use rga::args::*;
use rga::matching::*;
use ripgrep_all as rga;
use structopt::StructOpt;

use std::process::Command;

fn main() -> Fallible<()> {
    env_logger::init();

    let (args, passthrough_args) = split_args()?;

    if args.list_adapters {
        let (enabled_adapters, disabled_adapters) = get_all_adapters();

        println!("Adapters:\n");
        let print = |adapter: std::rc::Rc<dyn FileAdapter>| {
            let meta = adapter.metadata();
            let matchers = meta
                .fast_matchers
                .iter()
                .map(|m| match m {
                    FastMatcher::FileExtension(ext) => format!(".{}", ext),
                })
                .collect::<Vec<_>>()
                .join(", ");
            let slow_matchers = meta
                .slow_matchers
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| match m {
                    SlowMatcher::MimeType(x) => Some(format!("{}", x)),
                    SlowMatcher::Fast(_) => None,
                })
                .collect::<Vec<_>>()
                .join(", ");
            let mime_text = if slow_matchers.is_empty() {
                "".to_owned()
            } else {
                format!("Mime Types: {}", slow_matchers)
            };
            print!(
                " - {name}\n     {desc}\n     Extensions: {matchers}\n     {mime}\n",
                name = meta.name,
                desc = meta.description,
                matchers = matchers,
                mime = mime_text
            );
            println!("");
        };
        for adapter in enabled_adapters {
            print(adapter)
        }
        println!("The following adapters are disabled by default, and can be enabled using '--rga-adapters=+pdfpages,tesseract':\n");
        for adapter in disabled_adapters {
            print(adapter)
        }
        return Ok(());
    }

    if passthrough_args.len() == 0 {
        // rg would show help. Show own help instead.
        RgaArgs::clap().print_help()?;
        println!("");
        return Ok(());
    }

    let adapters = get_adapters_filtered(&args.adapters)?;

    let pre_glob = if !args.accurate {
        let extensions = adapters
            .iter()
            .flat_map(|a| &a.metadata().fast_matchers)
            .filter_map(|m| match m {
                FastMatcher::FileExtension(ext) => Some(ext as &str),
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("*.{{{}}}", extensions)
    } else {
        "*".to_owned()
    };

    let exe = std::env::current_exe().expect("Could not get executable location");
    let preproc_exe = exe.with_file_name("rga-preproc");

    let rg_args = vec![
        "--no-line-number",
        // smart case by default because within weird files
        // we probably can't really trust casing anyways
        "--smart-case",
    ];

    let mut child = Command::new("rg")
        .args(rg_args)
        .arg("--pre")
        .arg(preproc_exe)
        .arg("--pre-glob")
        .arg(pre_glob)
        .args(passthrough_args)
        .spawn()
        .map_err(|e| map_exe_error(e, "rg", "Please make sure you have ripgrep installed."))?;

    child.wait()?;
    Ok(())
}
