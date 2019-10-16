use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    platform::windows::WindowExtWindows,
};
use ash::{
    vk,
    Entry,
    version::EntryV1_0,
    vk_make_version,
    extensions::{
        ext::DebugReport,
        khr::{
            Surface,
            Win32Surface,
        },
    },
};
use std::{
    ptr::{null, null_mut},
    ffi::CString,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new()?;
    // let extension_properties = entry.enumerate_instance_extension_properties()?;
    // println!("Extensions: {:#?}", extension_properties);
    let layer_properties = entry.enumerate_instance_layer_properties()?;
    println!("layers: {:#?}", layer_properties);

    let app_info = vk::ApplicationInfo {
        api_version: vk_make_version!(1, 0, 0),
        ..Default::default()
    };
    let layer_names = [CString::new("VK_LAYER_LUNARG_standard_validation").unwrap()];
    let layers_names_raw: Vec<*const i8> = layer_names
                    .iter()
                    .map(|raw_name| raw_name.as_ptr())
                    .collect();
    let extension_names = vec![
        Surface::name().as_ptr(),
        Win32Surface::name().as_ptr(),
        DebugReport::name().as_ptr(),
    ];
    let extension_names_raw: Vec<*const i8> = extension_names
                    .iter()
                    .map(|raw_name| *raw_name)
                    .collect();
    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&extension_names_raw);

    let instance = unsafe { entry.create_instance(&create_info, None)? };


    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Experiment")
        .build(&event_loop).unwrap();
    // let hwnd = window.hwnd();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            _ => *control_flow = ControlFlow::Wait,
        }
    });
}
