use std::env;
use std::process::exit;

use termios::*;

use lc3::VM;


fn main() {
    let mut vm = VM::default();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("lc3 [image-file1] ...");
        exit(2);
    }

    args.iter().skip(1).for_each(|image| {
        if vm.load_file(image).is_err() {
            println!("failed to load image: {}", image);
            exit(1);
        }
    });

    // Get the terminal working such that it reads one char at a time.
    let stdin = 0;
    let mut termios = Termios::from_fd(stdin).unwrap();
    termios.c_iflag &= IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON;
    termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(stdin, TCSANOW, &mut termios).unwrap();

    vm.run();

    tcsetattr(stdin, TCSANOW, &termios).unwrap();
}