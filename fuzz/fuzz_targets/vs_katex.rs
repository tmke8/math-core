#![no_main]

use fantoccini::{ClientBuilder, Locator};
use image::{ImageFormat, ImageReader};
use libfuzzer_sys::fuzz_target;
use math_core::latex_to_mathml;
use std::io::Cursor;
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;

fuzz_target!(|data: &str| {
    // Parse with our parser
    let l2m = {
        if let Ok(l2m) = latex_to_mathml(data, math_core::Display::Block, false) {
            let l2m = l2m
                .strip_prefix(r#"<math display="block">"#)
                .unwrap()
                .strip_suffix("</math>")
                .unwrap()
                // work around minor display differences
                // we might want to fix these; I dunno
                .replace(r##" lspace="0.2222em""##, "")
                .replace(r##" rspace="0.2222em""##, "")
                .replace(r##" lspace="0em""##, "")
                .replace(r##" rspace="0em""##, "")
                .replace(r##" mathvariant="normal""##, "");
            format!(r##"<math xmlns="http://www.w3.org/1998/Math/MathML" display="block"><semantics><mrow>{l2m}</mrow><annotation encoding="application/x-tex">{data}</annotation></semantics></math>"##)
        } else {
            return;
        }
    };

    // Uncomment this to just hammer the parser without comparing it against anything.
    //return;

    // Parse with katex
    let katex = if let Ok(katex) =
        katex::render_with_opts(
            data,
            &katex::Opts::builder()
                .output_type(katex::OutputType::Mathml)
                .display_mode(true)
                .build()
                .unwrap()
        )
    {
        katex
            // remove pointless wrappers
            .strip_prefix("<span class=\"katex\">")
            .unwrap_or(&katex)
            .strip_suffix("</span>")
            .unwrap_or(&katex)
            // work around minor display differences
            // we might want to fix these; I dunno
            .replace(r##" lspace="0.2222em""##, "")
            .replace(r##" rspace="0.2222em""##, "")
            .replace(r##" lspace="0em""##, "")
            .replace(r##" rspace="0em""##, "")
            .replace(r##" mathvariant="normal""##, "")
            .to_owned()
    } else {
        return;
    };

    // Performance hack: we compare the katex and l2m mathml with the root tag
    // removed, because they put some of the attributes in the opposite order.
    // If they're identical, then none of this matters.
    {
        if l2m == katex {
            return;
        } else {
            println!("== need to check in a browser, because the mathml isn't identical");
            println!("data:          {data:?}");
            println!("trimmed l2m:   {l2m:?} {len}", len = l2m.len());
            println!("trimmed katex: {katex:?} {len}", len = katex.len());
        }
    }

    // Now pipe both of these out to Firefox, and see if they produce identical results.
    // For speed, the fuzz tester only checks with one browser.
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let port = 4445;
        let mut process = tokio::process::Command::new("geckodriver")
            .args(["--port", port.to_string().as_str()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let client = ClientBuilder::native()
            .connect(&format!("http://localhost:{}", port))
            .await?;

        // Wait for Firefox to start.
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        // Write l2m mathml to HTML template
        let mut tmp = tempfile::Builder::new().suffix(".html").tempfile()?;
        let path = format!("file://{}", tmp.path().to_str().unwrap());
        html_template(
            tmp.as_file_mut(),
            "l2m",
            |file: &mut std::fs::File| {
                file.write_all(l2m.as_bytes())?;
                Ok(())
            },
        )?;

        // Get l2m screenshot
        client.goto(&path).await?;
        let elem = client
            .wait()
            .at_most(Duration::from_secs(10))
            .for_element(Locator::XPath("/html/body"))
            .await?;
        let screenshot_l2m = elem.screenshot().await.ok().and_then(|screenshot| {
            ImageReader::new(Cursor::new(screenshot)).with_guessed_format().ok()?.decode().ok()
        });

        // Write katex mathml to HTML template
        let mut tmp = tempfile::Builder::new().suffix(".html").tempfile()?;
        let path = format!("file://{}", tmp.path().to_str().unwrap());
        html_template(
            tmp.as_file_mut(),
            "katex",
            |file: &mut std::fs::File| {
                file.write_all(katex.as_bytes())?;
                Ok(())
            },
        )?;

        // Get katex screenshot
        client.goto(&path).await?;
        let elem = client
            .wait()
            .at_most(Duration::from_secs(10))
            .for_element(Locator::XPath("/html/body"))
            .await?;
        let screenshot_katex = elem.screenshot().await.ok().and_then(|screenshot| {
            ImageReader::new(Cursor::new(screenshot)).with_guessed_format().ok()?.decode().ok()
        });

        client.close().await?;
        process.kill().await?;

        if screenshot_l2m != screenshot_katex {
            if let Some(screenshot_katex) = screenshot_katex {
                screenshot_katex.save_with_format("katex.png", ImageFormat::Png)?;
            }
            if let Some(screenshot_l2m) = screenshot_l2m {
                screenshot_l2m.save_with_format("l2m.png", ImageFormat::Png)?;
            }
            panic!();
        }

        Result::<(), Box<dyn std::error::Error>>::Ok(())
    }).unwrap();
});

fn html_template(
    file: &mut std::fs::File,
    title: &str,
    render: impl FnOnce(&mut std::fs::File) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    file.write_fmt(format_args!(
        r#"<!DOCTYPE html>
<html>
<head>
<title>{title}</title>
<meta charset="UTF-8">
<link rel="stylesheet" href="{source}/../tests/out/cross-browser-render.css">
</head>
<body>
"#,
        source = env!("CARGO_MANIFEST_DIR"),
    ))?;

    render(file)?;

    file.write_fmt(format_args!(
        r#"</body>
</html>"#
    ))?;
    Ok(())
}
