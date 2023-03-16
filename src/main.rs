use dll_syringe::{process::OwnedProcess, Syringe};

fn main() {
    let target_process = OwnedProcess::find_first_by_name("javaw.exe").unwrap();
    println!("{:?}", target_process);
    let syringe = Syringe::for_process(target_process);
    let injected_payload = syringe.inject("./target/debug/inject.dll").unwrap();
    println!("{:?}", injected_payload);

    // syringe.eject(injected_payload).unwrap();
    // Ok(())
}
