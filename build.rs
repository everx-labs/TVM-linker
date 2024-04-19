/*
 * Copyright 2018-2024 EverX Labs Ltd.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use std::process::Command;

fn from_utf8(data: Vec<u8>) -> String {
    match String::from_utf8(data) {
        Ok(string) => string,
        Err(_) => String::from("Unknown")
    }
}

fn main() {
    let mut git_branch = String::from("Unknown");
    let mut git_commit = String::from("Unknown");
    let mut commit_date = String::from("Unknown");
    let mut build_time = String::from("Unknown");

    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output();
    if let Ok(branch) = branch {
        git_branch = from_utf8(branch.stdout);
    }

    let last = Command::new("git").args(["rev-parse", "HEAD"]).output();
    if let Ok(last) = last {
        git_commit = from_utf8(last.stdout);
    }

    let time = Command::new("git")
        .args(["log", "-1", "--date=iso", "--pretty=format:%cd"])
        .output();
    if let Ok(time) = time {
        commit_date = from_utf8(time.stdout);
    }

    let b_time = Command::new("date").args(["+%Y-%m-%d %T %z"]).output();
    if let Ok(b_time) = b_time {
        build_time = from_utf8(b_time.stdout);
    }

    println!("cargo:rustc-env=BUILD_GIT_BRANCH={}", git_branch);
    println!("cargo:rustc-env=BUILD_GIT_COMMIT={}", git_commit);
    println!("cargo:rustc-env=BUILD_GIT_DATE={}", commit_date);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
}
