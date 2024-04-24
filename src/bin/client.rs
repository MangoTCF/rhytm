#[path = "../udde.rs"]
mod udde;
use core::result::Result::Ok;

use log::Level;
use udde::ClientMsgs;

use anyhow::{Context, Result};

use pyo3::{
    pyclass, pymethods,
    types::{IntoPyDict, PyModule, PyString},
    IntoPy, Python,
};

use std::os::unix::net::UnixDatagram;
use std::process::exit;
use std::{env, time::Duration};

/**
 * Print usage information and exit.
 */
fn usage(selfpath: String) {
    eprintln!(
        "Usage: {} <self socket path> <master socket path> <alphanumerical id>",
        selfpath
    );
}

#[pyclass]
#[derive(Debug)]
struct Callback {
    #[allow(dead_code)] // callback_function is called from Python
    callback_function: fn(&PyString, UnixDatagram),
    ud: UnixDatagram,
}

#[pymethods]
impl Callback {
    fn __call__(&self, d: &PyString) -> () {
        let _ = (self.callback_function)(
            d,
            self.ud
                .try_clone()
                .expect("Unable to clone datagram socket to callback function"),
        );
    }
}

/**
 * TODO: Make an init function and put all redundant code there
 * TODO: implement, accepts a self socket path, master socket path and thread id(?) as stdin args, starts download through yt_dlp, injecting callback into hooks, which communicates with master thread to update progress bars via FIFO
 */
fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 4 || args.len() > 5 {
        usage(args[0].clone());
        exit(1);
    }

    let socket = UnixDatagram::bind(args[1].clone()).expect(&format!("Unable to bind to socket @ {}", args[1].clone()));

    socket
        .connect(args[2].clone())
        .expect("Unable to connect to master socket");
    println!(
        "Sending {:?}",
        &serde_json::to_vec(&ClientMsgs::Greeting).unwrap()
    );
    socket
        .send(&serde_json::to_vec(&ClientMsgs::Greeting).unwrap())
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_millis(5000)))
        .expect("Unable to set socket timeout");

    let mut buf = Vec::<u8>::with_capacity(2048);
    buf.resize(2048, 0);
    let bytes = socket.recv(&mut buf).expect("Unable to recieve greeting");
    socket
        .set_write_timeout(None)
        .expect("Unable to reset socket timeout");

    if serde_json::from_slice::<ClientMsgs>(&buf[..bytes]).unwrap() != ClientMsgs::Greeting {
        unimplemented!(
            "Wrong greeting received, {:?} instead of {:?}",
            std::str::from_utf8(&buf[..bytes]).unwrap(),
            serde_json::to_string(&ClientMsgs::Greeting)
        );
    }

    pyo3::prepare_freethreaded_python();
    //TODO: Move redundant init code here

    loop {
        socket.send(&serde_json::to_vec(&ClientMsgs::BatchRequest).unwrap())?;
        let bytes = socket.recv(&mut buf).unwrap();
        match serde_json::from_slice::<ClientMsgs>(&buf[..bytes]).unwrap() {
            ClientMsgs::Greeting => {
                unimplemented!("Wrong batch header, Greeting instead of Batch possible server/client version mismatch")
            }
            ClientMsgs::Batch(batch) => {
                for link in batch {
                    Python::with_gil(|py| {
                        //Override stdout to disable _all_ output from Python code
                        let sys = py.import("sys").expect("Python: Unable to import sys");
                        sys.setattr(
                            "stdout",
                            py.eval("open(\"/dev/null\", \"w\")", Option::None, Option::None)
                                .expect("Python: Unable to open /dev/null"),
                        )
                        .expect("Python: Unable to set stdout to /dev/null");
                        sys.setattr(
                            "stderr",
                            py.eval("open(\"/dev/null\", \"w\")", Option::None, Option::None)
                                .expect("Python: Unable to open /dev/null"),
                        )
                        .expect("Python: Unable to set stderr to /dev/null");
                        let callback = Callback {
                            callback_function: |d, ud| {
                                ud.send(
                                    &serde_json::to_vec(&ClientMsgs::Log {
                                        thr_id: env::args().collect::<Vec<String>>()[3]
                                            .trim()
                                            .parse()
                                            .unwrap(),
                                        level: Level::Debug,
                                        target: "Thread".to_string(),
                                        msg: "Sending JSON!".to_string(),
                                    })
                                    .unwrap(),
                                )
                                .unwrap();
                                ud.send(&serde_json::to_vec(&ClientMsgs::JSON(d.to_string())).unwrap())
                                    .unwrap();

                                ud.send(&[udde::ClientMsgs::JSON as u8])
                                    .expect("Callback: Unable to send JSON header");
                                let str = d.to_str().expect("Callback: Unable to parse json string");
                                std::fs::write(
                                    "/home/mango/programming/rhytm/thr0.log",
                                    str.len().to_string().as_bytes(),
                                )
                                .unwrap();
                                ud.send(&str.len().to_ne_bytes())
                                    .expect("Callback: Unable to send JSON length");
                                match ud
                                    .send(str.as_bytes())
                                    .with_context(|| format!("length is {}", str.len()))
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        std::fs::write(
                                            "/home/mango/programming/rhytm/test_output/fucked-client.json",
                                            str.as_bytes(),
                                        )
                                        .unwrap();
                                        panic!("Callback: Unable to send json: {}", e)
                                    }
                                }
                            },
                            ud: socket
                                .try_clone()
                                .expect("Python: Unable to create a clone of socket to use in callback function"),
                        };

                        //Function for preprocessing JSON dictionary before sending it to Rust
                        //I know that this is fucked up but I am unable to figure out a better solution
                        let callback_preprocess = PyModule::from_code(
                            py,
                            "\n\
                    import json\n\
                    def preproc_hook(dict):\n\
                    \tfn(json.dumps(dict))",
                            "",
                            "",
                        )
                        .expect("Python: Unable to create preprocessor progress hook");

                        callback_preprocess
                            .setattr("fn", callback.into_py(py))
                            .expect("Python: Unable to set callback in preprocessor");

                        let params = vec![(
                            "cookiesfrombrowser",
                            (
                                "firefox",
                                Option::<&str>::None,
                                Option::<&str>::None,
                                Option::<&str>::None,
                            ),
                        )]
                        .into_py_dict(py);

                        params
                            .set_item("verbose", false)
                            .expect("Python: Unable to set verbose in yt-dlp params");
                        params
                            .set_item("quiet", true)
                            .expect("Python: Unable to set quiet in yt-dlp params");
                        params
                            .set_item("http_chunk_size", 10485760)
                            .expect("Python: Unable to set http_chunk_size in yt-dlp params");
                        params
                            .set_item("fragment_retries", 5)
                            .expect("Python: Unable to set fragment_retries in yt-dlp params");
                        params
                            .set_item(
                                "progress_hooks",
                                vec![callback_preprocess
                                    .getattr("preproc_hook")
                                    .expect("Python: We REALLY SHOULD NOT BE HERE, unable to get preprocessor progress hook")],
                            )
                            .expect("Python: Unable to set progress_hooks in yt-dlp params");
                        params
                            .set_item("simulate", false)
                            .expect("Python: Unable to set simulate in yt-dlp params");

                        let args = vec![("params", params)].into_py_dict(py);

                        let youtube_dl = py
                            .import("yt_dlp")
                            .expect("Failed to import yt_dlp")
                            .call_method("YoutubeDL", (), Some(args))
                            .expect("Python: Unable to create YoutubeDL object");

                        let _ = youtube_dl.call_method1("download", (link.clone(),));
                    });
                    socket
                        .send(
                            &serde_json::to_vec(&ClientMsgs::Log {
                                thr_id: env::args().collect::<Vec<String>>()[3]
                                    .trim()
                                    .parse()
                                    .unwrap(),
                                level: Level::Info,
                                target: "Thread".to_string(),
                                msg: format!("finished downloading video ID {}", link),
                            })
                            .unwrap(),
                        )
                        .unwrap();
                }
            }
            ClientMsgs::EndRequest => {
                break;
            }
            ClientMsgs::Log { .. } => unimplemented!("Wrong batch header, Log instead of Batch possible server/client version mismatch"),
            ClientMsgs::BatchRequest => unimplemented!("Wrong batch header, BatchRequest instead of Batch possible server/client version mismatch"),
            ClientMsgs::JSON(_) => unimplemented!("Wrong batch header, JSON instead of Batch possible server/client version mismatch"),
        }
    }

    Ok(())
}
