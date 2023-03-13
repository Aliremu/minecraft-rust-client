#![allow(non_snake_case)]
#![allow(temporary_cstring_as_ptr)]
// #![allow(warnings)]
// #![feature(trace_macros)]

// trace_macros!(true);

use jni::{
    sys::{jint, jsize, JNI_EDETACHED, JNI_OK, JNI_VERSION_1_6},
    JavaVM, AttachGuard, JNIEnv, objects::{JValue, JObject, JValueGen, JClass}
};
use std::{ffi::{CString, c_void}, os::windows::prelude::{FromRawHandle, IntoRawHandle}, fs::OpenOptions};
use winapi::{um::{
    // consoleapi::AllocConsole,
    libloaderapi::{GetModuleHandleA, GetProcAddress}, consoleapi::{AllocConsole, SetConsoleCtrlHandler}, processenv::GetStdHandle, memoryapi::{VirtualProtect, VirtualAlloc}, winnt::{PAGE_EXECUTE_READWRITE, MEM_RESERVE, MEM_COMMIT},
    // wincon::{AttachConsole, FreeConsole, ATTACH_PARENT_PROCESS},
}, shared::{winerror::FRS_ERR_CHILD_TO_PARENT_COMM, minwindef::DWORD}};

use std::fs::File;
use std::io::{BufWriter, Write};

use inject_derive::Inject;
use inject_derive::hack;

type GetJvms = unsafe extern "C" fn(
    vmBuf: *mut *mut jni::sys::JavaVM,
    bufLen: jsize,
    nVMs: *mut jsize,
) -> jint;

#[derive(Inject)]
#[hack]
struct Minecraft<'a> {
    app: App,

    #[class(name="ejf")]
    class: JClass<'a>,

    //Fields
    #[field(name="x", ty="I")]
    missTime: i32,

    #[field(name="aV", ty="I")]
    frames: i32,

    #[field(name="aR", ty="Z")]
    pause: bool,

    #[field(name="t", ty="Lfcz;")]
    player: LocalPlayer,

    //Methods
    #[method(name="m", sig="()I")]
    get_fps: i32,

    #[method(name="N", sig="()Lejf;", static="true")]
    get_instance: Minecraft,

    #[method(name="c", sig="(Z)V", args="pause: bool")]
    pauseGame: ()
}

#[derive(Inject)]
#[hack]
struct LocalPlayer<'a> {
    app: App,

    #[class(name="fcz")]
    class: JClass<'a>,

    #[field(name="cJ", ty="D")]
    x: f64,

    #[field(name="cK", ty="D")]
    y: f64,

    #[field(name="cL", ty="D")]
    z: f64,
}


#[derive(Inject)]
#[hack]
struct System<'a> {
    app: App,

    #[class(name="java/lang/System")]
    class: JClass<'a>,

    #[field(name="out", ty="Ljava/io/PrintStream;", static="true")]
    out: PrintStream,
}

#[derive(Inject)]
#[hack]
struct PrintStream<'a> {
    app: App,

    #[class(name="java/io/PrintStream")]
    class: JClass<'a>,

    #[method(name="println", sig="(Ljava/lang/String;)V", args="text: String")]
    println: ()
}

// #[derive(Clone)]
struct App {
    // jvm: *mut jni::sys::JavaVM,
    jvm: JavaVM,
    // env: *mut jni::sys::JNIEnv,
    // test: *mut c_void
}

unsafe fn hook(toHook: *mut c_void, ourFunc: *mut c_void, len: usize) -> *mut c_void {
    let MinLen: usize = 14;

    if len < MinLen { 
        return 0 as *mut c_void; 
    }

    let mut stub: [u8; 14] = [
        0xFF, 0x25, 0x00, 0x00, 0x00, 0x00, // jmp qword ptr [$+6]
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 // ptr
    ];

    let pTrampoline: *mut c_void = VirtualAlloc(0 as *mut c_void, len + 14, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);

    let dwOld: *mut DWORD = std::ptr::null_mut();
    VirtualProtect(toHook, len, PAGE_EXECUTE_READWRITE, dwOld);

    let retto: u64 = toHook as u64 + len as u64;

    // trampoline
    std::ptr::copy_nonoverlapping(&retto, (stub.as_mut_ptr() as u8 + 6) as *mut u64, 8);
    std::ptr::copy_nonoverlapping(toHook, (pTrampoline as *mut DWORD) as *mut c_void, len);
    std::ptr::copy_nonoverlapping((stub.as_mut_ptr()) as *mut u64, (pTrampoline as usize + len) as *mut u64, 14);

    // orig
    std::ptr::copy_nonoverlapping(ourFunc, (stub.as_mut_ptr() as u8 + 6) as *mut c_void, 8);
    std::ptr::copy_nonoverlapping((stub.as_mut_ptr()) as *mut c_void, toHook, 14);

    for i in MinLen..len {
        *((toHook as usize + i) as *mut u8) = 0x90;
    }

    VirtualProtect(toHook, len, *dwOld, dwOld);
    return (pTrampoline as u32) as *mut c_void;
}

unsafe fn trampoline(toHook: *mut c_void, ourFunc: *mut c_void, len: usize) -> *mut c_void {
    // Make sure the length is greater than 5
    if len < 5 { return 0 as *mut c_void; }

    // Create the gateway (len + 5 for the overwritten bytes + the jmp)
    let gateway: *mut c_void = VirtualAlloc(0 as *mut c_void, len + 5, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);

    //Write the stolen bytes into the gateway
    std::ptr::copy_nonoverlapping(toHook, gateway, len);

    // Get the gateway to destination addy
    let gatewayRelativeAddr: i32 = (toHook as i32 - gateway as i32) - 5;

    // Add the jmp opcode to the end of the gateway
    *((gateway as i32 + len as i32) as *mut u8) = 0xE9;

    // Add the address to the jmp
    *((gateway as i32 + len as i32 + 1) as *mut i32) = gatewayRelativeAddr;

    // Perform the detour
    hook(toHook, ourFunc, len);

    gateway
}

impl App {
    pub unsafe fn get_env(&mut self) -> Result<JNIEnv, jni::errors::Error> {
        self.jvm.get_env()
    }
    pub unsafe fn new() -> Result<App, jni::errors::Error> {
        // AllocConsole();
        // SetConsoleCtrlHandler(None, 1);

        // let file = OpenOptions::new().write(true).read(true).open("CONOUT$").unwrap();
        // let err = winapi::um::processenv::SetStdHandle(winapi::um::winbase::STD_OUTPUT_HANDLE, file.into_raw_handle());

        // std::io::stdout().write("PEEPEE POOPOO".as_bytes()).unwrap();


        let mut jvm = std::ptr::null_mut();
        let mut count = 0;
        
        let GetCreatedJavaVMs = GetProcAddress(
            GetModuleHandleA(CString::new("jvm.dll").unwrap().as_ptr()),
            CString::new("JNI_GetCreatedJavaVMs").unwrap().as_ptr(),
        ) as *const usize;
    
        let JNI_GetCreatedJavaVMs: GetJvms = std::mem::transmute(GetCreatedJavaVMs);
    
        JNI_GetCreatedJavaVMs(&mut jvm, 1, &mut count);
        let jvm = JavaVM::from_raw(jvm)?;
        jvm.attach_current_thread()?;
        // (**jvm).AttachCurrentThread.unwrap()(jvm, &mut env, std::ptr::null_mut());

        Ok(Self {
            jvm: jvm
        })

        // let jvm = JavaVM::from_raw(jvm)?;
        // let mut guard = jvm.attach_current_thread()?;

        // Ok(Self {
        //     jvm: jvm,
        //     env: std::mem::transmute(env),
        //     test: 0 as *mut c_void
        // })
    }

    pub unsafe fn println(&mut self, text: &str) -> Result<(), jni::errors::Error> {
        let mut env = JNIEnv::from_raw(self.env)?;
        let system = env.find_class("java/lang/System")?;
        let print_stream = env.find_class("java/io/PrintStream")?;
        let out = env.get_static_field(system, "out", "Ljava/io/PrintStream;")?.l()?;
        let message = env.new_string("Hello World2")?;
        // env.set_static_field(class, field, value)
        let msg = env.new_string(text.to_string())?;
        // env.set_field(class, field, value)
        env
        .call_method(
            &out, 
            "println", 
            "(Ljava/lang/String;)V", 
            &[JValue::from(&msg)]
        )?;

        Ok(())
    }
   
    unsafe fn runTick(&mut self) -> Result<(), jni::errors::Error> {
        self.println("RUNNING HOOK TICK")?;

        std::mem::transmute::<*mut c_void, fn()>(self.test)();

        Ok(())
    }

    pub unsafe fn hookTick(&mut self) -> Result<(), jni::errors::Error> {
        let mut env = JNIEnv::from_raw(self.env)?;

        let class = env.find_class("ejf")?;
        let method = env.get_method_id(&class, "s", "()V")?;
        
        let cum = Self::runTick as *mut c_void;

        self.println(format!("{:?} {:?}", method.into_raw(), cum).as_str())?;

        self.test = hook(method.into_raw() as *mut c_void, cum, 0x40);

        self.println(format!("{:?}", self.test).as_str())?;

        // std::mem::transmute::<*const (), fn(&mut App) -> Result<(), jni::errors::Error>>(cum)(self)?;
        // std::mem::transmute::<*const c_void, fn()>(method.into_raw() as *mut c_void)();

        // hook(method.into_raw() as *mut c_void, Self::runTick as *mut c_void, 5);

        Ok(())
    }
}


unsafe extern "stdcall" fn inject_thread() -> Result<(), jni::errors::Error> {
    // FreeConsole();
    // AllocConsole();
    // AttachConsole(ATTACH_PARENT_PROCESS);
    
    let mut app = App::new()?;
    app.println("SUCK MY NUTS")?;

    let mut mc = Minecraft::new(app)?;
    let mut mc = mc.get_instance_static()?;

    let mut player = mc.get_player()?;
    app.hookTick()?;
    player.
    // app.println(format!("{} {} {}", player.get_x()?, player.get_y()?, mc.get_pause()?).as_str())?;
    
    let mut system = System::new(app)?;
    let mut out = system.get_out_static()?;

    // out.println("text".to_string())?;
    
    // app.println(mc.get_fps().to_string().as_str())?;
    // let player = EntityPlayer::new(&mut guard);

    // let instance = Minecraft::new(&mut guard);
    
    // let system = guard.find_class("java/lang/System")?;
    // let print_stream = guard.find_class("java/io/PrintStream")?;

    // let out = guard.get_static_field(system, "out", "Ljava/io/PrintStream;")?.l()?;
    // let message = guard.new_string("Hello World2")?;

    // let Minecraft = guard.find_class("ejf")?;
    // let MinecraftInstance = guard.call_static_method(
    //     Minecraft, 
    //     "N",
    //     "()Lejf;",
    //     &[])?.l()?;


    // let LocalPlayerClass = guard.find_class("fcz")?;
    // let Player = guard.get_field(
    //     &MinecraftInstance, 
    //     "t",
    //     "Lfcz;")?.l()?;

    // //loop {
    //     let xPos = guard.get_field(&Player, "cJ", "D")?.d()?;

    //     let fps = guard.call_method(&MinecraftInstance, "m", "()I", &[])?.i()?;//guard.get_field(MinecraftInstance, "bb", "I")?.i()?;
    //     let msg = guard.new_string(xPos.to_string())?;
    //     guard
    //     .call_method(
    //         &out, 
    //         "println", 
    //         "(Ljava/lang/String;)V", 
    //         &[JValue::from(&msg)]
    //     )?;
    // //}

    // jvm.detach_current_thread();
    Ok(())
}

#[ctor::ctor]
unsafe fn ctor() {
    println!("Hi from library!");

    unsafe {
        let handle = std::thread::spawn(|| inject_thread());
    }
}

#[ctor::dtor]
unsafe fn dtor() {
    println!("Closing!");
}