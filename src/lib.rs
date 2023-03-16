#![allow(non_snake_case)]
#![allow(temporary_cstring_as_ptr)]
// #![allow(warnings)]
// #![feature(trace_macros)]

// trace_macros!(true);

use jni::{
    sys::{jint, jsize},
    JavaVM, JNIEnv, objects::{JValue, JObject, JClass}
};
use core::time;
use std::{ffi::{CString, c_void}, thread, time::Duration};
use winapi::{um::{
    libloaderapi::{GetModuleHandleA, GetProcAddress, FreeLibraryAndExitThread}, memoryapi::{VirtualProtect, VirtualAlloc}, winnt::{PAGE_EXECUTE_READWRITE, MEM_RESERVE, MEM_COMMIT}, consoleapi::AllocConsole,
}, shared::{minwindef::DWORD}};

use inject_derive::Inject;
use inject_derive::inject;
use detour::static_detour;

static_detour! {
    static Test: /* extern "X" */ fn() -> i32;
    static CREEP: /* extern "X" */ fn(i32) -> i32;
}

fn add5(val: i32) -> i32 {
  val + 5
}

fn add10(val: i32) -> i32 {
  val + 210
}

type GetJvms = unsafe extern "C" fn(
    vmBuf: *mut *mut jni::sys::JavaVM,
    bufLen: jsize,
    nVMs: *mut jsize,
) -> jint;


#[derive(Inject)]
#[inject]
struct PoseStack<'a> {
    app: &'a App,

    #[class(name="eed")]
    class: JClass<'a>,
}

#[derive(Inject)]
#[inject]
struct InteractionHand<'a> {
    app: &'a App,

    #[class(name="bcl")]
    class: JClass<'a>,

    //Fields
    #[field(name="a", ty="Lbcl;", static="true")]
    MAIN_HAND: InteractionHand,

    #[field(name="b", ty="Lbcl;", static="true")]
    OFF_HAND: InteractionHand,
}

#[derive(Inject)]
#[inject]
struct Font<'a> {
    app: &'a App,

    #[class(name="ekm")]
    class: JClass<'a>,

    #[method(name="b", sig="(Leed;Ljava/lang/String;FFI)I")]
    draw: fn(stack: &PoseStack, text: &str, x: f32, y: f32, color: i32) -> i32
}

#[derive(Inject)]
#[inject]
struct Camera<'a> {
    app: &'a App,

    #[class(name="eir")]
    class: JClass<'a>,

    #[field(name="j", ty="F")]
    xRot: f32,

    #[field(name="k", ty="F")]
    yRot: f32,

    #[method(name="a", sig="(FF)V")]
    setRotation: fn(x: f32, y: f32) -> ()
}

#[derive(Inject)]
#[inject]
struct GameRenderer<'a> {
    app: &'a App,

    #[class(name="fdo")]
    class: JClass<'a>,

    #[field(name="M", ty="Leir;")]
    mainCamera: Camera
}

#[derive(Inject)]
#[inject]
struct Minecraft<'a> {
    app: &'a App,

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

    #[field(name="h", ty="Lekm;")]
    font: Font,

    #[field(name="j", ty="Lfdo;")]
    gameRenderer: GameRenderer,

    //Methods
    #[method(name="m", sig="()I")]
    get_fps: fn() -> i32,

    #[method(name="N", sig="()Lejf;", static="true")]
    get_instance: fn() -> Minecraft,

    #[method(name="c", sig="(Z)V")]
    pauseGame: fn(pause: bool) -> ()
}


#[derive(Inject)]
#[inject]
struct Abilities<'a> {
    app: &'a App,

    #[class(name="bwm")]
    class: JClass<'a>,

    #[field(name="b", ty="Z")]
    flying: bool,

    #[field(name="c", ty="Z")]
    mayfly: bool
}

#[derive(Inject)]
#[inject]
struct LocalPlayer<'a> {
    app: &'a App,

    #[class(name="fcz")]
    class: JClass<'a>,

    #[field(name="cJ", ty="D")]
    x: f64,

    #[field(name="cK", ty="D")]
    y: f64,

    #[field(name="cL", ty="D")]
    z: f64,

    #[field(name="cM", ty="F")]
    yRotLast: f32,

    #[field(name="cN", ty="F")]
    xRotLast: f32,

    #[field(name="cq", ty="Lbwm;")]
    abilities: Abilities,

    #[method(name="x", sig="(F)V")]
    hurtTo: fn(hurt: f32) -> (),

    #[method(name="a", sig="(Lbcl;)V")]
    swing: fn(hand: InteractionHand) -> (),
}


#[derive(Inject)]
#[inject]
struct System<'a> {
    app: &'a App,

    #[class(name="java/lang/System")]
    class: JClass<'a>,

    #[field(name="out", ty="Ljava/io/PrintStream;", static="true")]
    out: PrintStream,
}

#[derive(Inject)]
#[inject]
struct PrintStream<'a> {
    app: &'a App,

    #[class(name="java/io/PrintStream")]
    class: JClass<'a>,

    #[method(name="println", sig="(Ljava/lang/String;)V")]
    println: fn(text: &str) -> (),
}

#[derive(Inject)]
#[inject]
struct LoggedPrintStream<'a> {
    app: &'a App,

    #[class(name="acm")]
    class: JClass<'a>,

    #[method(name="println", sig="(Ljava/lang/String;)V")]
    println: fn(text: &str) -> (),
}

struct App {
    jvm: JavaVM
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
    pub unsafe fn get_env(&self) -> Result<JNIEnv, jni::errors::Error> {
        self.jvm.get_env()
    }

    pub unsafe fn new() -> Result<App, jni::errors::Error> {
        let mut jvm = std::ptr::null_mut();
        let mut count = 0;
        
        let GetCreatedJavaVMs = GetProcAddress(
            GetModuleHandleA(CString::new("jvm.dll").unwrap().as_ptr()),
            CString::new("JNI_GetCreatedJavaVMs").unwrap().as_ptr(),
        ) as *const usize;
    
        let JNI_GetCreatedJavaVMs: GetJvms = std::mem::transmute(GetCreatedJavaVMs);
    
        JNI_GetCreatedJavaVMs(&mut jvm, 1, &mut count);
        let jvm = JavaVM::from_raw(jvm)?;
        jvm.attach_current_thread_permanently()?;

        Ok(Self {
            jvm: jvm
        })
    }

    pub unsafe fn println(&self, text: &str) -> Result<(), jni::errors::Error> {
        let mut env = self.get_env()?;
        let system = env.find_class("java/lang/System")?;
        let print_stream = env.find_class("java/io/PrintStream")?;
        let out = env.get_static_field(system, "out", "Ljava/io/PrintStream;")?.l()?;
        
        let msg = env.new_string(text.to_string())?;

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

        // std::mem::transmute::<*mut c_void, fn()>(self.test)();
        // Test.call(true);

        Ok(())
    }

    pub unsafe fn hookTick(&self) -> Result<(), jni::errors::Error> {
        let mut env = self.get_env()?;
        
        let class = env.find_class("ejf")?;
        let method = env.get_method_id(&class, "m", "()I")?;
        self.println(format!("wtf us this {}", *(*(method.into_raw() as *mut u64) as *mut u64)).as_str())?;
        self.println(format!("wtf us this {}", (method.into_raw() as u64)).as_str())?;
        self.println(format!("wtf us this {}", std::ptr::read(method.into_raw() as *mut u64)).as_str())?;
        
        Test.initialize(std::mem::transmute(
            *((*(method.into_raw() as *mut u64) + 0x40) as *mut u64)
        ), wtf).unwrap();
        Test.enable().unwrap();
        // let cum = Self::runTick as *mut c_void;
        // Test.initialize(std::mem::transmute::<*const c_void, fn(bool)>(method.into_raw() as *mut c_void), wtf).unwrap();
        // Test.enable().unwrap();
        self.println("HOOKED")?;

        // self.println(format!("{:?} {:?}", method.into_raw(), cum).as_str())?;

        // self.test = trampoline(method.into_raw() as *mut c_void, cum, 0x40);

        // self.println(format!("{:?}", self.test).as_str())?;

        // std::mem::transmute::<*const (), fn(&mut App) -> Result<(), jni::errors::Error>>(cum)(self)?;
        // std::mem::transmute::<*const c_void, fn()>(method.into_raw() as *mut c_void)();

        // hook(method.into_raw() as *mut c_void, Self::runTick as *mut c_void, 5);

        Ok(())
    }
}


pub fn wtf() -> i32 {
    69
}

unsafe fn inject_thread() -> Result<(), jni::errors::Error> {
    let mut app = App::new()?;
    app.println("SUCK MY wqeqweqweNUTS")?;
    let mut mc = Minecraft::new(&app)?;
    let mut mc = mc.get_instance_static()?;

    let mut renderer = mc.get_gameRenderer()?;
    let mut camera = renderer.get_mainCamera()?;
    // camera.setRotation(90.0, 90.0)?;

    // let mut font = mc.get_font()?;

    // let mut stack = PoseStack::new(&app)?;
    // let stack_instance = app.get_env()?.new_object(&stack.class, "()V", &[])?;
    // stack.set_instance(stack_instance);

    // for i in 0..1000 {
    //     font.draw(&stack, "rewrwer wer werwe rwerwer wer", 0.0, 0.0, 0xFFFFFF)?;
    // }
    let mut player = mc.get_player()?;
    // // app.hookTick()?;
    player.hurtTo(5.0)?;
    for i in 0..100 {
        player.swing(InteractionHand::new(&app)?.get_OFF_HAND_static()?)?;
        player.get_abilities()?.set_mayfly(true)?;
        player.get_abilities()?.set_flying(true)?;
        app.println(format!("{} {} {}", camera.get_xRot()?, player.get_yRotLast()?, mc.get_pause()?).as_str())?;
        
        
        thread::sleep(std::time::Duration::from_millis(50));
    }
    // let mut system = System::new(&app)?;
    // let mut out = system.get_out_static()?;
    // out.println("Think fast chuckle nuts")?;

    app.jvm.detach_current_thread();
    FreeLibraryAndExitThread(GetModuleHandleA(0 as *const i8), 0);
    Ok(())
}

#[ctor::ctor]
unsafe fn ctor() {
    println!("Attaching to thread!");

    unsafe {
        let handle = std::thread::spawn(|| inject_thread());
    }
}

#[ctor::dtor]
unsafe fn dtor() {
    println!("Bye bye!");
}