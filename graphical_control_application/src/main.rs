// Just blatently copy and paste examples from here: 
// https://github.com/gyscos/cursive/tree/main/examples

use std::process::Command;


use openssh::*;

#[tokio::main]
async fn main() {


    Command::new("ssh-agent")
    .arg("-s").output().expect("eh... ssh-agent didn't execute properly...");

    let output = Command::new("ssh-add").arg("/home/elliot/.ssh/digial_ocean").output().expect("eh... ssh-add didn't execute properly...");


    let s = String::from_utf8_lossy(&output.stdout);

    print!("rustc succeeded and stdout was:\n{}", s);
    // let mut siv = cursive::default();

    // // We'll use a counter to name new files.
    // let counter = AtomicUsize::new(1);

    // // The menubar is a list of (label, menu tree) pairs.

    // siv.add_layer(Dialog::text("Hit <Esc> to show the menu!"));

    // siv.run();



    let session = Session::connect("ssh://root@134.122.12.127:22", KnownHosts::Accept)
        .await
        .unwrap();

    let ls = session.command("ls").output().await.unwrap();
    eprintln!(
        "{}",
        String::from_utf8(ls.stdout).expect("server output was not valid UTF-8")
    );

}
