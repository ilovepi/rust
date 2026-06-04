// Verifies that direct-access-external-data, PIC, and PIE flags are passed and
// correctly set in LTO backends, and external variables are dso_local.
//
//@ ignore-loongarch64
//@ ignore-powerpc64
//@ ignore-apple
//
//@ revisions: DEFAULT-THIN DEFAULT-FULL PIE-THIN PIE-FULL DIRECT-THIN DIRECT-FULL INDIRECT-THIN INDIRECT-FULL

// Default static relocation model, no direct-access. Should be dso_local.
//@ [DEFAULT-THIN] compile-flags: -C relocation-model=static -Clto=thin
//@ [DEFAULT-THIN] filecheck-flags: --check-prefix=DSO_LOCAL --check-prefix=NO_FLAG --check-prefix=NO_PIE_LEVELS
//@ [DEFAULT-FULL] compile-flags: -C relocation-model=static -Clto=fat
//@ [DEFAULT-FULL] filecheck-flags: --check-prefix=DSO_LOCAL --check-prefix=NO_FLAG --check-prefix=NO_PIE_LEVELS

// PIE relocation model, no direct-access override. Should not be dso_local, has PIE levels.
//@ [PIE-THIN] compile-flags: -C relocation-model=pie -Clto=thin
//@ [PIE-THIN] filecheck-flags: --check-prefix=NO_DSO_LOCAL --check-prefix=NO_FLAG --check-prefix=PIE_LEVELS
//@ [PIE-FULL] compile-flags: -C relocation-model=pie -Clto=fat
//@ [PIE-FULL] filecheck-flags: --check-prefix=NO_DSO_LOCAL --check-prefix=NO_FLAG --check-prefix=PIE_LEVELS

// PIE relocation model + direct-access=yes. Should be dso_local, has direct-access flag = 1, has PIE levels.
//@ [DIRECT-THIN] compile-flags: -C relocation-model=pie -Z direct-access-external-data=yes -Clto=thin
//@ [DIRECT-THIN] filecheck-flags: --check-prefix=DSO_LOCAL --check-prefix=DIRECT_FLAG --check-prefix=PIE_LEVELS
//@ [DIRECT-FULL] compile-flags: -C relocation-model=pie -Z direct-access-external-data=yes -Clto=fat
//@ [DIRECT-FULL] filecheck-flags: --check-prefix=DSO_LOCAL --check-prefix=DIRECT_FLAG --check-prefix=PIE_LEVELS

// Static relocation model + direct-access=no. Should not be dso_local, has direct-access flag = 0.
//@ [INDIRECT-THIN] compile-flags: -C relocation-model=static -Z direct-access-external-data=no -Clto=thin
//@ [INDIRECT-THIN] filecheck-flags: --check-prefix=NO_DSO_LOCAL --check-prefix=INDIRECT_FLAG --check-prefix=NO_PIE_LEVELS
//@ [INDIRECT-FULL] compile-flags: -C relocation-model=static -Z direct-access-external-data=no -Clto=fat
//@ [INDIRECT-FULL] filecheck-flags: --check-prefix=NO_DSO_LOCAL --check-prefix=INDIRECT_FLAG --check-prefix=NO_PIE_LEVELS

#![crate_type = "lib"]

unsafe extern "C" {
    // CHECK: @VAR = external
    // DSO_LOCAL-SAME: dso_local
    // NO_DSO_LOCAL-NOT: dso_local
    // CHECK-SAME: global i32
    safe static VAR: i32;
}

#[no_mangle]
pub fn refer() -> i32 {
    VAR
}

// DIRECT_FLAG-DAG: !{{[0-9]+}} = !{i32 7, !"direct-access-external-data", i32 1}
// INDIRECT_FLAG-DAG: !{{[0-9]+}} = !{i32 7, !"direct-access-external-data", i32 0}
// NO_FLAG-NOT: direct-access-external-data

// PIE_LEVELS-DAG: !{{[0-9]+}} = !{i32 8, !"PIC Level", i32 2}
// PIE_LEVELS-DAG: !{{[0-9]+}} = !{i32 7, !"PIE Level", i32 2}
// NO_PIE_LEVELS-NOT: PIE Level
