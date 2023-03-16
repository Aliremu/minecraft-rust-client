# minecraft-rust-client

Minecraft Injection Client built in Rust

## Usage/Examples

Define classes using procedural macros
```rust
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
```

Fly Example

```rust
unsafe fn main() {
    let mut app = App::new()?;
    app.println("Injected!")?;
    let mut mc = Minecraft::new(&app)?;
    let mut mc = mc.get_instance_static()?;

    let mut player = mc.get_player()?;
    
    player.get_abilities()?.set_mayfly(true)?;
}
```

## Source Mappings
You can find official source mappings from Mojang in each version's .json file.