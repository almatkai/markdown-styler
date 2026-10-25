use tezbar_markdown_styler::{convert, document};
#[test]
fn exact_own_tsx_marked_916_oracle() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/oracle/oracle.json")).unwrap();
    let mut failures = Vec::new();
    for c in cases.as_array().unwrap() {
        let got = convert(c["input"].as_str().unwrap()).unwrap();
        if got.styled_html != c["styledHtml"] {
            failures.push(c["name"].as_str().unwrap());
            let temp = tempfile::tempdir().unwrap();
            let tmp = temp.path();
            std::fs::write(
                tmp.join(format!("{}-actual.html", c["name"].as_str().unwrap())),
                &got.styled_html,
            )
            .unwrap();
            std::fs::write(
                tmp.join(format!("{}-expected.html", c["name"].as_str().unwrap())),
                c["styledHtml"].as_str().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(got.markdown, c["markdown"], "{}", c["name"]);
        assert_eq!(
            serde_json::to_value(&got.preview_utf16).unwrap(),
            c["previewUtf16"],
            "{}",
            c["name"]
        );
        if !got.styled_html.is_empty() {
            assert_eq!(
                document::original(c["styledHtml"].as_str().unwrap(), false),
                c["document"],
                "template {}",
                c["name"]
            )
        }
        // The lone-surrogate preview is intentionally visibly escaped by UTF-8 adapter.
        if c["name"] != "preview" {
            assert_eq!(
                got.display("10:13:20 PM", "4fzzzxjyl"),
                c["display"],
                "{}",
                c["name"]
            );
        }
    }
    assert!(failures.is_empty(), "Rendering mismatches: {failures:?}");
    assert_eq!(
        tezbar_markdown_styler::render::styled(document::SAMPLE).unwrap(),
        include_str!("../fixtures/oracle/sample-styled.html")
    );
}
