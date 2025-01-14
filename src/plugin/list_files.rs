// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use client;
use client::RpcCaller;
use error::Result;
use plugin;
use rpc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert;
use std::fs::{self, DirEntry};
use std::io;
use std::mem;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;
use time;

enum Continue {
    Yes,
    No,
}

// NOCOM(#sirver): rewrite
// one possible implementation of fs::walk_dir only visiting files
fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&DirEntry) -> Continue) -> io::Result<Continue> {
    if fs::metadata(dir)?.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let filetype = entry.file_type()?;
            if filetype.is_dir() && !filetype.is_symlink() {
                match visit_dirs(&entry.path(), cb)? {
                    Continue::Yes => (),
                    Continue::No => return Ok(Continue::No),
                }
            } else {
                match cb(&entry) {
                    Continue::Yes => (),
                    Continue::No => return Ok(Continue::No),
                }
            }
        }
    }
    Ok(Continue::Yes)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesUpdate {
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesRequest {
    pub directory: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListFilesResponse;

struct ListFiles {
    client: Arc<RwLock<client::ThinClient>>,
}

impl client::rpc::server::Rpc for ListFiles {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: ListFilesRequest = try_rpc!(context, serde_json::from_value(args));
        // NOCOM handle the result
        let _ = self.client.write().unwrap().call(
            "log.debug",
            &plugin::log::debug::Request {
                message: String::from("list files called"),
                time: plugin::log::current(),
            },
        );

        thread::spawn(move || {
            let mut files = Vec::new();
            let mut last = time::SteadyTime::now();
            visit_dirs(Path::new(&request.directory), &mut |entry| {
                if context.cancelled() {
                    return Continue::No;
                }

                files.push(entry.path().to_string_lossy().into_owned());
                let now = time::SteadyTime::now();
                if now - last > time::Duration::milliseconds(50) {
                    last = now;
                    if context
                        .update(&ListFilesUpdate {
                            files: mem::replace(&mut files, Vec::new()),
                        })
                        .is_err()
                    {
                        return Continue::No;
                    };
                }
                Continue::Yes
            })
            .unwrap();

            // Ignore errors: we might have been cancelled.
            let _ = context.update(&ListFilesUpdate {
                files: mem::replace(&mut files, Vec::new()),
            });
            let response = ListFilesResponse;
            let _ = context.finish(rpc::Result::success(response));
        });
    }
}

pub struct Plugin {
    _client: client::Client,
}

impl Plugin {
    pub fn new(mut client: client::Client) -> Result<Self> {
        let thin_client = Arc::new(RwLock::new(client.clone()?));
        plugin::register_rpc(
            &mut client,
            rpc_map! {
                "list_files" => ListFiles { client: thin_client.clone() },
            }
        )?;
        Ok(Plugin { _client: client })
    }
}
