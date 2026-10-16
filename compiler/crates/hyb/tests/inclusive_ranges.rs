//! Inclusive ranges must terminate at an integer type's maximum, including
//! through `continue`. Exercise checked source through emitted, executed Go.
//! Every maximum-value loop has an explicit three-iteration guard so the
//! regression also terminates on the original broken compiler.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_CASE: AtomicUsize = AtomicUsize::new(0);

struct Case(PathBuf);

impl Case {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hyb-inclusive-range-{}-{}",
            std::process::id(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated case");
        let case = Self(root);
        fs::write(case.0.join("program.hy"), source).expect("write source");
        case
    }

    fn run(&self) -> String {
        let binary = format!("program{}", std::env::consts::EXE_SUFFIX);
        for args in [
            vec!["check", "program.hy"],
            vec!["build", "program.hy", "-o", binary.as_str()],
        ] {
            let out = Command::new(env!("CARGO_BIN_EXE_hyb"))
                .args(args)
                .current_dir(&self.0)
                .output()
                .expect("run hyb");
            assert_success(&out);
        }
        let out = Command::new(self.0.join(binary))
            .current_dir(&self.0)
            .output()
            .expect("run emitted program");
        assert_success(&out);
        String::from_utf8(out.stdout)
            .expect("UTF-8 stdout")
            .replace("\r\n", "\n")
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn inclusive_integer_maxima_run_once() {
    let int_max = if cfg!(target_pointer_width = "64") {
        "9223372036854775807"
    } else {
        "2147483647"
    };
    let uint_max = if cfg!(target_pointer_width = "64") {
        "18446744073709551615"
    } else {
        "4294967295"
    };
    for (ty, maximum) in [
        ("int8", "127"),
        ("uint8", "255"),
        ("int16", "32767"),
        ("uint16", "65535"),
        ("int32", "2147483647"),
        ("uint32", "4294967295"),
        ("int64", "9223372036854775807"),
        ("uint64", "18446744073709551615"),
        ("int", int_max),
        ("uint", uint_max),
    ] {
        let source = format!(
            r#"import "fmt"
fn main() {{
    var high {ty} = {maximum}
    count := 0
    for value in high..=high {{
        fmt.println(value)
        count++
        if count == 3 {{ break }}
    }}
    fmt.println(count)
}}
"#
        );
        assert_eq!(
            Case::new(&source).run(),
            format!("{maximum}\n1\n"),
            "inclusive maximum for {ty}"
        );
    }
}

#[test]
fn continue_at_the_maximum_still_terminates() {
    for continuation in ["continue", "continue loop"] {
        let source = format!(
            r#"import "fmt"
fn main() {{
    var high uint8 = 255
    count := 0
    for value in high..=high {{
        count++
        if count == 3 {{ break }}
        {continuation}
    }}
    fmt.println(count)
}}
"#
        );
        assert_eq!(Case::new(&source).run(), "1\n", "{continuation}");
    }
}

#[test]
fn ordinary_empty_and_exclusive_ranges_preserve_bound_evaluation() {
    let source = r#"import "fmt"
fn lower(calls []int) int { calls[0] += 1; return 0 }
fn upper(calls []int) int { calls[1] += 1; return 1 }
fn main() {
    calls := []int{0, 0}
    count := 0
    for value in lower(calls)..=upper(calls) {
        fmt.println(value)
        count++
        if count == 3 { break }
    }
    fmt.println(count)
    fmt.println(calls[0])
    fmt.println(calls[1])
    for value in 2..=1 { count++ }
    fmt.println(count)
    for value in 0..2 { fmt.println(value); count++ }
    fmt.println(count)
}
"#;
    assert_eq!(Case::new(source).run(), "0\n1\n2\n1\n3\n2\n0\n1\n4\n");
}

#[test]
fn overflow_temporary_does_not_capture_source_names_or_rewrite_strings() {
    let source = r#"import "fmt"
fn main() {
    var hyRangeOverflow0 int8 = 127
    count := 0
    for hyRangeOverflow4 in hyRangeOverflow0..=hyRangeOverflow0 {
        var item hyRangeOverflow2
        item.value = hyRangeOverflow1()
        fmt.println(item.value)
        fmt.println("hyRangeOverflow3")
        fmt.println(hyRangeOverflow4)
        count++
        if count == 3 { break }
    }
    fmt.println(count)
}
fn hyRangeOverflow1() int { return 7 }
struct hyRangeOverflow2 { value int }
"#;
    assert_eq!(Case::new(source).run(), "7\nhyRangeOverflow3\n127\n1\n");
}

#[test]
fn nested_inclusive_ranges_keep_separate_termination_state() {
    let source = r#"import "fmt"
fn main() {
    var hyRangeOverflow0 int8 = 127
    outer := 0
    total := 0
    for first in hyRangeOverflow0..=hyRangeOverflow0 {
        outer++
        inner := 0
        for second in hyRangeOverflow0..=hyRangeOverflow0 {
            total += hyRangeOverflow1()
            inner++
            if inner == 3 { break }
        }
        if outer == 3 { break }
    }
    fmt.println(outer)
    fmt.println(total)
}
fn hyRangeOverflow1() int { return 1 }
"#;
    assert_eq!(Case::new(source).run(), "1\n1\n");
}
