#[path = "../udde.rs"]
mod udde;
use log::Level;
use udde::server_msgs;

use anyhow::{Ok, Result};

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

    let socket = UnixDatagram::bind(args[1].clone())
        .expect(&format!("Unable to bind to socket @ {}", args[1].clone()));

    socket
        .connect(args[2].clone())
        .expect("Unable to connect to master socket");

    socket.send(&[udde::client_msgs::Greeting as u8])?;
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("Unable to set socket timeout");
    let mut hbuf = [0; 1];
    socket.recv(&mut hbuf).expect("Unable to recieve greeting");
    socket
        .set_read_timeout(None)
        .expect("Unable to reset socket timeout");
    match num_traits::FromPrimitive::from_u8(hbuf[0])
        .expect("Wrong greeting, possible server/client version mismatch")
    {
        server_msgs::Greeting => {}
        _ => {
            unimplemented!("Wrong greeting, possible server/client version mismatch")
        }
    }

    let mut sbuf = [0 as u8; 2048];
    pyo3::prepare_freethreaded_python();
    loop {
        socket.send(&[udde::client_msgs::BatchRequest as u8])?;
        socket.recv(&mut hbuf)?;
        match num_traits::FromPrimitive::from_u8(hbuf[0])
            .expect("Wrong batch header, possible server/client version mismatch")
        {
            server_msgs::Greeting => {
                unimplemented!("Wrong batch header, possible server/client version mismatch")
            }
            server_msgs::Batch => {}
            server_msgs::EndRequest => {
                break;
            }
        }
        socket.recv(&mut sbuf)?;
        for i in sbuf.split(|x| x.to_owned() == b'\n') {
            let link = std::str::from_utf8(i).expect("Unable to parse link");
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
                        ud.send(&[udde::client_msgs::JSON as u8])
                            .expect("Callback: Unable to send JSON header");
                        let str = d.to_str().expect("Callback: Unable to parse json string");
                        ud.send(&str.len().to_ne_bytes())
                            .expect("Callback: Unable to send JSON length");
                        ud.send(str.as_bytes())
                            .expect("Callback: Unable to send json");
                    },
                    ud: socket.try_clone().expect(
                        "Python: Unable to create a clone of socket to use in callback function",
                    ),
                };

                //Function for preprocessing JSON dictionary before sending it to Rust
                //I know that this is fucked up but I am unable to figure out a better solution
                let callback_preprocess = PyModule::from_code(
                    py,
                    "\n\
                    import json\n\
                    def preproc_hook(dict):\n\
                    \tprint(\"calling callback\")\n\
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
                params.set_item(
                    "progress_hooks",
                    vec![callback_preprocess
                        .getattr("preproc_hook")
                        .expect("Python: We REALLY SHOULD NOT BE HERE, unable to get preprocessor progress hook")],
                ).expect("Python: Unable to set progress_hooks in yt-dlp params");
                params
                    .set_item("simulate", false)
                    .expect("Python: Unable to set simulate in yt-dlp params");

                let args = vec![("params", params)].into_py_dict(py);

                let youtube_dl = py
                    .import("yt_dlp")
                    .expect("Failed to import yt_dlp")
                    .call_method("YoutubeDL", (), Some(args))
                    .expect("Python: Unable to create YoutubeDL object");

                let _ = youtube_dl.call_method1("download", (link,));
            });
            socket.send(&[udde::client_msgs::Log as u8])?;
            socket.send(Level::Info.as_str().as_bytes())?;
            let msg = format!("Thread {} finished downloading video ID {}", args[3], link);
            socket.send(&msg.len().to_ne_bytes())?;
            socket.send(&msg.as_bytes())?;
        }

        //cleaning up buffers
        sbuf.fill(0);
    }

    Ok(())
}
