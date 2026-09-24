# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Production callback and cross-crate async/declaration admission controls."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,function,diagnostic", [
    ("completion", "KeeperUnit::restore", "postcondition not satisfied"),
    ("cancel", "check_keeper_cancellation", "assertion failed"),
    ("unawaited", "check_keeper_unawaited", "assertion failed"),
    ("domain", "check_keeper_requires_domain", "precondition not satisfied"),
    ("stopped", "check_keeper_requires_stopped", "precondition not satisfied"),
    ("saved_value", "check_keeper_completion", "postcondition not satisfied"),
])
def test_production_keeper_rejects_invalid_claims(case, function, diagnostic, tmp_path):
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "vmm_core", "--", "--verify-only-module", "vmtime_unit",
        "--verify-function", function, "--cfg", f"verus_keeper_negative_{case}",
        "--rlimit", "50", "--num-threads", "1",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid production callback claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,function,diagnostic", [
    ("clock_value", "check_local_clock_fields", "assertion failed"),
    ("clock_started", "check_local_clock_fields", "assertion failed"),
    ("save_started", "check_started_save", "precondition not satisfied"),
])
def test_production_local_clock_rejects_invalid_claims(case, function, diagnostic, tmp_path):
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "vmcore", "--", "--verify-only-module", "vmtime",
        "--verify-function", function, "--cfg", f"verus_keeper_negative_{case}",
        "--rlimit", "50", "--num-threads", "1",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid local clock claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


TRAIT = """
use vstd::prelude::*;
pub trait Restore {
    fn diagnostic(&self, error: std::io::Error);
    #[verus_verify]
    #[verus_spec(result => ensures result == saved,)]
    async fn restore(&mut self, saved: u64) -> u64;
    #[verus_verify]
    #[verus_spec(result => ensures result == saved,)]
    async fn other(&mut self, saved: u64) -> u64;
}
"""

CONSUMER = """
use vstd::prelude::*;
use restore_trait::Restore;
#[verus_verify]
pub struct Keeper<'a> { pub time: &'a mut u64 }
impl Restore for Keeper<'_> {
    fn diagnostic(&self, _error: std::io::Error) {}
    #[cfg(not(wrong_output))]
    #[verus_spec(result => ensures
        *final(self).time == saved,
        *final(final(self).time) == *final(old(self).time),
        cfg!(wrong_ensures) ==> result == 37,
    )]
    async fn restore(&mut self, saved: u64) -> u64 {
        *self.time = saved;
        saved
    }
    #[cfg(wrong_output)]
    #[verus_verify]
    async fn restore(&mut self, _saved: u64) -> bool { false }
    #[cfg_attr(not(missing_verified), verus_spec(result => ensures result == saved,))]
    async fn other(&mut self, saved: u64) -> u64 { saved }
}
verus! {
async fn complete() {
    let mut time = 5u64;
    {
        let mut keeper = Keeper { time: &mut time };
        let future = keeper.restore(13);
        #[cfg(unawaited)]
        assert(vstd::future::FutureAdditionalSpecFns::awaited(&future));
        #[cfg(borrow_conflict)]
        { *keeper.time = 7; }
        let result = future.await;
        assert(result == 13);
    }
    assert(time == 13);
}
#[cfg(cancelled)]
fn cancel() {
    let mut time = 5u64;
    {
        let mut keeper = Keeper { time: &mut time };
        let _future = keeper.restore(13);
    }
    assert(time == 13);
}
#[cfg(opaque_identity)]
fn require_same<T>(a: T, b: T) {}
#[cfg(opaque_identity)]
fn different_futures(a: &mut Keeper<'_>, b: &mut Keeper<'_>) {
    require_same(a.restore(13), b.other(13));
}
}
"""


@pytest.fixture(scope="module")
def async_library(tmp_path_factory):
    directory = tmp_path_factory.mktemp("keeper-async")
    (directory / "trait.rs").write_text(TRAIT)
    (directory / "consumer.rs").write_text(CONSUMER)
    result = subprocess.run([
        os.environ["VERUS"], "--crate-type", "lib", "--crate-name", "restore_trait",
        "--compile", "--export", str(directory / "restore_trait.vir"),
        "--out-dir", str(directory), "--rlimit", "50", str(directory / "trait.rs"),
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (directory / "export.log").write_text(result.stdout)
    assert result.returncode == 0, result.stdout
    return directory


@pytest.mark.parametrize("case,diagnostic", [
    ("positive", None),
    ("wrong_ensures", "postcondition not satisfied"),
    ("wrong_output", "E0271"),
    ("borrow_conflict", "E0506"),
    ("unawaited", "assertion failed"),
    ("cancelled", "assertion failed"),
    ("opaque_identity", "E0308"),
    ("missing_verified", "an item in a trait impl cannot be marked external"),
])
def test_imported_async_contracts(async_library, case, diagnostic):
    directory = async_library
    flags = [] if case == "positive" else ["--cfg", case]
    result = subprocess.run([
        os.environ["VERUS"], "--crate-type", "lib", "--rlimit", "50",
        "--extern", f"restore_trait={directory / 'librestore_trait.rlib'}",
        "--import", f"restore_trait={directory / 'restore_trait.vir'}",
        *flags, str(directory / "consumer.rs"),
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (directory / f"{case}.log").write_text(result.stdout)
    assert "panicked" not in result.stdout, result.stdout
    if diagnostic is None:
        assert result.returncode == 0, result.stdout
        assert "3 verified, 0 errors" in result.stdout, result.stdout
    else:
        assert result.returncode != 0, "invalid imported async claim verified"
        assert diagnostic in result.stdout, result.stdout


@pytest.mark.parametrize("case,body,diagnostic", [
    ("identity", "value", None),
    ("construct", "Opaque::Empty", "opaque"),
    ("match", "match value { Opaque::Error(_) | Opaque::Empty => value }", "opaque"),
    ("clone", "let got = value.clone(); proof! { assert(got == value); } got", "assertion failed"),
])
def test_opaque_declaration_grants_no_implementation(case, body, diagnostic, tmp_path):
    source = tmp_path / "opaque.rs"
    source.write_text("""
use vstd::prelude::*;
#[derive(Clone)]
#[verus_verify(external_body)]
pub enum Opaque { Error(std::sync::Arc<std::io::Error>), Empty }
#[verus_verify]
pub fn operation(value: Opaque) -> Opaque {
""" + body + "\n}\n")
    result = subprocess.run([
        os.environ["VERUS"], "--crate-type", "lib", "--rlimit", "50", str(source),
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert "panicked" not in result.stdout, result.stdout
    if diagnostic is None:
        assert result.returncode == 0, result.stdout
        assert "1 verified, 0 errors" in result.stdout, result.stdout
    else:
        assert result.returncode != 0, result.stdout
        assert diagnostic in result.stdout, result.stdout


@pytest.fixture(scope="module")
def alias_library(tmp_path_factory):
    directory = tmp_path_factory.mktemp("keeper-alias")
    source = directory / "handles.rs"
    source.write_text("""
use vstd::prelude::*;
pub struct Handle<T, M> { pub value: T, pub metadata: M }
impl<T: Copy, M> Handle<T, M> {
    pub fn read(&self) -> T { self.value }
}
pub struct Other<T, M> { pub value: T, pub metadata: M }
""")
    result = subprocess.run([
        os.environ["VERUS"], "--crate-type", "lib", "--crate-name", "handles",
        "--compile", "--out-dir", str(directory), str(source),
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (directory / "build.log").write_text(result.stdout)
    assert result.returncode == 0, result.stdout
    return directory


@pytest.mark.parametrize("case,body,diagnostic", [
    ("identity", """
#[verus_verify]
fn identity<T>(task: handles::Handle<T, u8>) -> Task<T> { task }
#[verus_verify]
#[verus_spec(result => ensures result == owner.clock,)]
fn observe(owner: &Owner) -> u64 { owner.clock }
""", None),
    ("wrong_field", """
#[verus_verify]
#[verus_spec(result => ensures result == owner.clock,)]
fn observe(owner: &Owner) -> u64 { owner.other }
""", "postcondition not satisfied"),
    ("opaque_field", """
#[verus_verify]
fn observe(task: &Task<u64>) -> u64 { task.value }
""", "opaque"),
    ("opaque_constructor", """
#[verus_verify]
fn construct(value: u64) -> Task<u64> {
    handles::Handle { value, metadata: 0 }
}
""", "opaque"),
    ("unproved_operation", """
#[verus_verify]
fn observe(task: &Task<u64>) -> u64 { task.read() }
""", "not supported"),
    ("source_identity", """
#[verus_verify]
fn identity<T>(task: handles::Other<T, u8>) -> Task<T> { task }
""", "E0308"),
    ("lifetime", """
#[verus_verify]
fn escape<'a>(task: Task<&'a u64>) -> Task<&'static u64> { task }
""", "lifetime may not live long enough"),
    ("borrow", """
#[verus_verify]
fn conflict(owner: &mut Owner) -> u64 {
    let before = &*owner;
    owner.clock = 7;
    before.clock
}
""", "E0506"),
    ("extensional_equality", """
#[verus_verify(external_body)]
#[verifier::ext_equal]
type OtherTask<T> = handles::Other<T, u8>;
""", "cannot replace a Verus datatype or grant extensional equality"),
    ("local_alias", """
struct Local { value: u64 }
#[verus_verify(external_body)]
type LocalAlias = Local;
""", "must resolve directly to a foreign struct"),
    ("reference_alias", """
#[verus_verify(external_body)]
type Reference<'a> = &'a u64;
""", "must resolve directly to a foreign struct"),
])
def test_source_bound_opaque_alias(alias_library, case, body, diagnostic):
    directory = alias_library
    source = directory / f"{case}.rs"
    source.write_text("""
use vstd::prelude::*;
#[verus_verify(external_body)]
type Task<T> = handles::Handle<T, u8>;
#[verus_verify]
struct Owner { task: Task<u64>, clock: u64, other: u64 }
""" + body)
    result = subprocess.run([
        os.environ["VERUS"], "--crate-type", "lib", "--rlimit", "50",
        "--extern", f"handles={directory / 'libhandles.rlib'}", str(source),
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (directory / f"{case}.log").write_text(result.stdout)
    assert "panicked" not in result.stdout, result.stdout
    if diagnostic is None:
        assert result.returncode == 0, result.stdout
        assert "2 verified, 0 errors" in result.stdout, result.stdout
    else:
        assert result.returncode != 0, "invalid alias use verified"
        assert diagnostic in result.stdout, result.stdout
