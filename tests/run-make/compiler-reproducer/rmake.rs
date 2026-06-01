// This test checks that the unstable `-Zcrash-diagnostics` compiler reproducer packaging
// correctly captures all input files, precompiled dependencies, and target specs on panics,
// and respects the `-Zcrash-diagnostics=off` disable switch.

use std::path::{Path, PathBuf};

use run_make_support::{
    cmd, cwd, has_prefix, has_suffix, rfs, run_in_tmpdir, rustc, serde_json, shallow_find_files,
};

fn find_single_entry<F>(dir: &Path, filter: F, error_msg: &str) -> PathBuf
where
    F: Fn(&Path) -> bool,
{
    let entries =
        rfs::read_dir(dir).map(|e| e.unwrap().path()).filter(|p| filter(p)).collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "{}: {:?}", error_msg, dir);
    entries[0].clone()
}

fn get_single_dir(dir: &Path) -> PathBuf {
    find_single_entry(dir, |p| p.is_dir(), "Expected exactly one subdirectory in")
}

fn get_reproducer_dir(out_dir: &Path) -> PathBuf {
    get_single_dir(out_dir)
}

fn verify_reproducer(
    out_dir: &Path,
    expected_srcs: &[(&str, &Path)],
    expected_deps: &[(&str, &Path)],
    expected_sh_args: &[&str],
) {
    let bundle_dir = get_reproducer_dir(out_dir);

    let metadata_path = bundle_dir.join("metadata.json");
    let reproduce_sh_path = bundle_dir.join("reproduce.sh");

    assert!(metadata_path.exists(), "metadata.json missing");
    assert!(reproduce_sh_path.exists(), "reproduce.sh missing");

    for (orig_name, rel_path) in expected_srcs {
        let copied_src = bundle_dir.join(rel_path);
        assert!(copied_src.exists(), "Expected copied source file {:?} missing", rel_path);
        assert_eq!(
            rfs::read_to_string(orig_name),
            rfs::read_to_string(copied_src),
            "Source file contents mismatched for {:?}",
            orig_name
        );
    }

    for (crate_name, rel_path) in expected_deps {
        let copied_dep = bundle_dir.join(rel_path);
        assert!(copied_dep.exists(), "Expected copied dependency {:?} missing", rel_path);
    }

    let meta_str = rfs::read_to_string(&metadata_path);
    let meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
    assert!(meta.get("rustc_version").is_some(), "Missing rustc_version in metadata");
    assert!(meta.get("platform").is_some(), "Missing platform in metadata");
    assert!(meta.get("timestamp").is_some(), "Missing timestamp in metadata");
    assert!(meta.get("pid").is_some(), "Missing pid in metadata");

    let env = meta.get("env").expect("Missing env in metadata");
    assert!(env.get("RUSTFLAGS").is_some(), "Missing RUSTFLAGS in env metadata");
    assert!(env.get("RUSTC_LOG").is_some(), "Missing RUSTC_LOG in env metadata");

    let sh_str = rfs::read_to_string(&reproduce_sh_path);
    let mut args = Vec::new();
    for line in sh_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("export ") {
            continue;
        }
        let mut arg = line.strip_suffix(" \\").unwrap_or(line).to_string();
        arg = arg.trim_matches('\'').trim_matches('"').to_string();
        args.push(arg);
    }

    let cmd_rustc = Path::new(&args[0]);
    assert!(cmd_rustc.is_absolute(), "Expected absolute rustc path, got {:?}", cmd_rustc);

    for expected_arg in expected_sh_args {
        let expected = expected_arg.trim_matches('\'').trim_matches('"');
        assert!(
            args.iter().any(|a| a == expected),
            "Missing expected arg {:?} in reproduce.sh: {:?}",
            expected,
            args
        );
    }

    for arg in &args {
        assert!(
            !arg.contains("crash-diagnostics-dir"),
            "Reproducers cannot be recursive. Found {:?} in reproduce.sh",
            arg
        );
    }

    assert!(
        args.contains(&"-Zcrash-diagnostics=off".to_string()),
        "Missing -Zcrash-diagnostics=off in reproduce.sh"
    );
}

fn main() {
    // Helper to map a file in temp test dir to its expected bundle relative path
    let map_path = |filename: &str| -> PathBuf {
        let abs_cwd = cwd().canonicalize().unwrap();
        let mut components = abs_cwd.components();
        components.next(); // skip RootDir
        components.as_path().join(filename)
    };

    // Verify that a standard ICE during binary compilation successfully packages a reproducer.
    run_in_tmpdir(|| {
        let out_dir = cwd();
        let dir_arg = format!("-Zcrash-diagnostics-dir={}", out_dir.display());

        let output = rustc().input("lib.rs").arg("-Ztreat-err-as-bug=1").arg(&dir_arg).run_fail();

        output.assert_stderr_contains("compiler reproducer bundle successfully generated");

        let expected_src = map_path("lib.rs");

        verify_reproducer(
            &out_dir,
            &[("lib.rs", &expected_src)],
            &[],
            &[&expected_src.to_string_lossy(), "-Ztreat-err-as-bug=1"],
        );
    });

    // Verify that reproducing a library target crash bundles the source files correctly.
    run_in_tmpdir(|| {
        let out_dir = cwd();
        let dir_arg = format!("-Zcrash-diagnostics-dir={}", out_dir.display());

        let output = rustc()
            .input("lib.rs")
            .crate_type("lib")
            .arg("-Ztreat-err-as-bug=1")
            .arg(&dir_arg)
            .run_fail();

        output.assert_stderr_contains("compiler reproducer bundle successfully generated");

        let expected_src = map_path("lib.rs");

        verify_reproducer(
            &out_dir,
            &[("lib.rs", &expected_src)],
            &[],
            &[&expected_src.to_string_lossy(), "--crate-type", "lib", "-Ztreat-err-as-bug=1"],
        );
    });

    // Verify that crate dependencies are resolved and bundled.
    run_in_tmpdir(|| {
        let out_dir = cwd();
        let dir_arg = format!("-Zcrash-diagnostics-dir={}", out_dir.display());

        rustc().input("dep.rs").crate_type("rlib").out_dir(&out_dir).run();

        let libdep_name = run_make_support::rust_lib_name("dep");
        let libdep_path = out_dir.join(&libdep_name);
        assert!(libdep_path.exists(), "libdep.rlib was not compiled");

        rustc()
            .input("bar.rs")
            .crate_type("rlib")
            .extern_("dep", &libdep_path)
            .out_dir(&out_dir)
            .run();

        let libbar_name = run_make_support::rust_lib_name("bar");
        let libbar_path = out_dir.join(&libbar_name);
        assert!(libbar_path.exists(), "libbar.rlib was not compiled");

        let output = rustc()
            .input("main.rs")
            .extern_("bar", &libbar_path)
            .arg("-Ztreat-err-as-bug=1")
            .arg(&dir_arg)
            .run_fail();

        output.assert_stderr_contains("compiler reproducer bundle successfully generated");

        let expected_main = map_path("main.rs");
        let expected_foo = map_path("foo.rs");
        let expected_bar_dep = map_path(&libbar_name);
        let expected_dep_dep = map_path(&libdep_name);

        verify_reproducer(
            &out_dir,
            &[("main.rs", &expected_main), ("foo.rs", &expected_foo)],
            &[("bar", &expected_bar_dep), ("dep", &expected_dep_dep)],
            &[
                &expected_main.to_string_lossy(),
                "--extern",
                &format!("bar={}", expected_bar_dep.to_string_lossy()),
                "--extern",
                &format!("dep={}", expected_dep_dep.to_string_lossy()),
                "-Ztreat-err-as-bug=1",
            ],
        );
    });

    // Verify that -Zcrash-diagnostics=off successfully disables packaging and creates no archives.
    run_in_tmpdir(|| {
        let out_dir = cwd();
        let dir_arg = format!("-Zcrash-diagnostics-dir={}", out_dir.display());

        let output = rustc()
            .input("lib.rs")
            .arg("-Ztreat-err-as-bug=1")
            .arg("-Zcrash-diagnostics=off")
            .arg(&dir_arg)
            .run_fail();

        output.assert_stderr_not_contains("compiler reproducer bundle successfully generated");

        let files = rfs::read_dir(&out_dir)
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .collect::<Vec<_>>();
        assert!(
            files.is_empty(),
            "No reproducer files or directories should be created when disabled"
        );
    });
}
