#[path = "../comms.rs"]
mod comms;
use core::result::Result::Ok;

use comms::{Message, MessageRead, MessageWrite};
use log::Level;

use anyhow::Result;

use pyo3::{
    pyclass, pymethods,
    types::{IntoPyDict, PyAnyMethods, PyModule, PyString, PyStringMethods},
    Bound, IntoPy, Python,
};

use std::env;
use std::os::unix::net::UnixStream;

#[pyclass]
#[derive(Debug)]
struct Callback {
    #[allow(dead_code)] // callback_function is called from Python
    callback_function: fn(&Bound<PyString>, UnixStream),
    ud: UnixStream,
}

#[pymethods]
impl Callback {
    fn __call__(&self, d: &Bound<PyString>) -> () {
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
 * TODO: implement, accepts a self socket path, master socket path and thread id(?) as stdin args,
 * starts download through yt_dlp, injecting callback into hooks, which communicates with master thread to update progress bars via FIFO
 */
fn main() -> Result<()> {
    let msp = env::var("MSP").expect("MSP not set");
    let thr_id = env::var("THR_ID")
        .expect("THR_ID not set")
        .parse::<usize>()
        .unwrap();
    let log_dir = env::var("LOG_DIR").expect("LOG_DIR not set");
    let download_dir = env::var("DOWNLOAD_DIR").expect("DOWNLOAD_DIR not set");
    let tmp_dir = env::var("TMP_DIR").expect("TMP_DIR not set");

    let mut socket = UnixStream::connect(msp.clone()).expect(&format!("Unable to bind to socket @ {}", msp));

    socket.write_json_msg(&Message::Greeting(thr_id)).unwrap();

    let msg = socket.read_json_msg::<Message>().unwrap();

    if msg != Message::Greeting(0) {
        print!("Received non-greeting message from server: {:?}", msg);
        panic!("Server sent garbage");
    }

    if let Message::Greeting(v) = msg {
        if thr_id != v {
            panic!("Server sent greeting with wrong id");
        }
    } else {
        panic!("How the fuck are we getting here?");
    }

    pyo3::prepare_freethreaded_python();
    //TODO: Move redundant init code here
    Python::with_gil(|py| {
        //Override stdout to disable _all_ output from Python code
        let sys = py
            .import_bound("sys")
            .expect("Python: Unable to import sys");
        sys.setattr(
            "stdout",
            py.eval_bound("open(\"/dev/null\", \"w\")", Option::None, Option::None)
                .expect("Python: Unable to open /dev/null"),
        )
        .expect("Python: Unable to set stdout to /dev/null");
        sys.setattr(
            "stderr",
            py.eval_bound("open(\"/dev/null\", \"w\")", Option::None, Option::None)
                .expect("Python: Unable to open /dev/null"),
        )
        .expect("Python: Unable to set stderr to /dev/null");
        let callback = Callback {
            callback_function: |d, mut ud| {
                ud.write_json_msg(&Message::Log {
                    thr_id: env::var("THR_ID")
                        .expect("THR_ID not set")
                        .parse::<usize>()
                        .unwrap(),
                    level: Level::Debug,
                    target: "Thread".to_string(),
                    msg: "Sending JSON!".to_string(),
                })
                .unwrap();

                let str = d.to_str().expect("Callback: Unable to parse json string");

                ud.write_json_msg(&Message::JSON(str.to_string()))
                    .expect("Callback: Unable to send JSON");
            },
            ud: socket
                .try_clone()
                .expect("Python: Unable to create a clone of socket to use in callback function"),
        };

        //Function for preprocessing JSON dictionary before sending it to Rust
        //I know that this is fucked up but I am unable to figure out a better solution
        let callback_preprocess = PyModule::from_code_bound(
            py,
            "\n\
                import json\n\
                def preproc_hook(dict):\n\
                \tfn(json.dumps(dict))",
            "",
            "",
        )
        .unwrap();

        callback_preprocess
            .setattr("fn", callback.into_py(py))
            .unwrap();

        let params = vec![(
            "cookiesfrombrowser",
            (
                "firefox",
                Option::<&str>::None,
                Option::<&str>::None,
                Option::<&str>::None,
            ),
        )]
        .into_py_dict_bound(py);

        params
            .set_item("verbose", false)
            .unwrap();
        params
            .set_item("quiet", true)
            .unwrap();
        params
            .set_item("http_chunk_size", 10485760)
            .unwrap();
        params
            .set_item("fragment_retries", 5)
            .unwrap();
        params
            .set_item(
                "progress_hooks",
                vec![callback_preprocess
                    .getattr("preproc_hook")
                    .unwrap()],
            )
            .unwrap();
        params
            .set_item("simulate", false)
            .expect("Python: Unable to set simulate in yt-dlp params");

        let args = vec![("params", params)].into_py_dict_bound(py);

        let youtube_dl = py
            .import_bound("yt_dlp")
            .expect("Failed to import yt_dlp")
            .call_method("YoutubeDL", (), Some(&args))
            .expect("Python: Unable to create YoutubeDL object");

        loop {
            socket.write_json_msg(&Message::BatchRequest).unwrap();
            match socket.read_json_msg::<Message>().unwrap() {
                Message::Greeting(_) => {
                    unimplemented!("Wrong batch header, Greeting instead of Batch possible server/client version mismatch")
                }
                Message::Batch(batch) => {
                    for link in batch {
                        let _ = youtube_dl.call_method1("download", (link.clone(),));

                        socket
                            .write_json_msg(&Message::Log {
                                thr_id,
                                level: Level::Info,
                                target: "Thread".to_string(),
                                msg: format!("finished downloading video ID {}", link),
                            })
                            .unwrap();
                    }
                }
                Message::EndRequest => {
                    break;
                }
                Message::Log { .. } => unimplemented!("Wrong batch header, Log instead of Batch, possible server/client version mismatch"),
                Message::BatchRequest => unimplemented!("Wrong batch header, BatchRequest instead of Batch, possible server/client version mismatch"),
                Message::JSON(_) => unimplemented!("Wrong batch header, JSON instead of Batch, possible server/client version mismatch"),
            }
        }
    });

    Ok(())
}
