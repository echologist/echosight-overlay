fn main() {
    let (status, json, result) = echosight_theme_cli::run(std::env::args_os().skip(1));
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("JSON result")
        );
    } else {
        if let Some(help) = result["help"].as_str() {
            print!("{help}");
        } else if let Some(themes) = result["themes"].as_array() {
            for theme in themes {
                println!(
                    "{}\t{}\tv{}\t{}",
                    theme["id"], theme["name"], theme["format"], theme["source"]
                );
            }
        } else if status == 0 {
            println!(
                "{}: OK (v{}) {}",
                result["command"], result["format"], result["id"]
            );
            if result["command"] == "convert" {
                println!(
                    "Structural conversion only, not visual equivalence. Review CONVERSION-REPORT.md in {}.",
                    result["output"]
                );
            }
        }
        if let Some(diagnostics) = result["diagnostics"].as_array() {
            for d in diagnostics {
                eprintln!(
                    "{} [{}] {}: {}",
                    d["severity"], d["code"], d["location"], d["message"]
                );
            }
        }
    }
    std::process::exit(status);
}
