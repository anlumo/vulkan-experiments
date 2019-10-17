use log::{info, error, debug, trace, log};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::LogicalSize,
    platform::windows::WindowExtWindows,
};
use ash::{
    vk,
    Entry,
    version::{
        DeviceV1_0,
        EntryV1_0,
        InstanceV1_0,
    },
    vk_make_version,
    extensions::{
        ext::{
            DebugReport,
            DebugUtils,
        },
        khr::{
            Surface,
            Win32Surface,
        },
    },
};
use std::ffi::{CString, CStr};

mod queue_families;
use crate::queue_families::QueueFamilyIndices;

extern "system" fn debug_messenger_callback(message_severity: vk::DebugUtilsMessageSeverityFlagsEXT, message_types: vk::DebugUtilsMessageTypeFlagsEXT, p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT, _p_user_data: *mut std::ffi::c_void) -> vk::Bool32 {
    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };

    let level = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => log::Level::Debug,
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => log::Level::Info,
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::Level::Warn,
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::Level::Error,
        _ => log::Level::Trace,
    };
    let module = match message_types {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "GENERAL",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "PERFORMANCE",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "VALIDATION",
        _ => "UNKNOWN",
    };

    log!(target: module, level, "{}", message.to_string_lossy());

    false.into()
}

fn is_device_suitable(instance: &ash::Instance, device: vk::PhysicalDevice) -> (u32, String) {
    let properties = unsafe { instance.get_physical_device_properties(device) };
    // let features = unsafe { instance.get_physical_device_features(device) };

    (properties.limits.max_image_dimension2_d + match properties.device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
        vk::PhysicalDeviceType::INTEGRATED_GPU => 500,
        vk::PhysicalDeviceType::VIRTUAL_GPU => 250,
        _ => 0,
    }, unsafe { CStr::from_ptr(&properties.device_name as *const i8) }.to_string_lossy().to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("log.yaml", Default::default())?;
    info!("Startup");

    let entry = Entry::new()?;
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
        DebugUtils::name().as_ptr(),
    ];
    let extension_names_raw: Vec<*const i8> = extension_names
        .iter()
        .map(|raw_name| *raw_name)
        .collect();
    let mut debug_messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
        .pfn_user_callback(Some(debug_messenger_callback));
    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_layer_names(&layers_names_raw)
        .enabled_extension_names(&extension_names_raw)
        .push_next(&mut debug_messenger_info);

    let instance = unsafe { entry.create_instance(&create_info, None)? };
    let debug_utils = DebugUtils::new(&entry, &instance);
    let debug_utils_messenger = unsafe { debug_utils.create_debug_utils_messenger(&debug_messenger_info, None)? };

    let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
    let physical_device = physical_devices.into_iter().filter_map(|device| {
        let (suitability, name) = is_device_suitable(&instance, device);
        if suitability > 0 {
            let indices = QueueFamilyIndices::find(&instance, device);
            if indices.is_device_suitable() {
                return Some((suitability, device, name, indices));
            }
        }
        None
    }).max_by_key(|val| val.0);
    if physical_device.is_none() {
        error!("No suitable graphics card found.");
        return Ok(());
    }
    let physical_device = physical_device.unwrap();
    info!("Device selected: {}", physical_device.2);

    let graphics_queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(physical_device.3.graphics.unwrap())
        .queue_priorities(&[1.0]);
    let queue_create_info = [*graphics_queue_create_info];
    let physical_device_features = vk::PhysicalDeviceFeatures::builder();
    
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_info)
        .enabled_features(&physical_device_features)
        .enabled_layer_names(&layers_names_raw);
    
    let device = unsafe { instance.create_device(physical_device.1, &device_create_info, None) }?;

    let graphics_queue = unsafe { device.get_device_queue(physical_device.3.graphics.unwrap(), 0) };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Experiment")
        .build(&event_loop).unwrap();
    // let hwnd = window.hwnd();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::EventsCleared => {
                trace!("Events cleared");
                // update state here
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id,
            } if window_id == window.id() => {
                trace!("redraw");
                // redraw here
            }
            Event::WindowEvent {
                event: WindowEvent::HiDpiFactorChanged(_dpi),
                window_id,
            } if window_id == window.id() => {
                // TODO
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(LogicalSize { width: _width, height: _height }),
                window_id,
            } if window_id == window.id() => {
                debug!("Window resized");
                // TODO
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                info!("Close requested");
                *control_flow = ControlFlow::Exit;
                unsafe {
                    device.device_wait_idle().unwrap();
                    device.destroy_device(None);
                    debug_utils.destroy_debug_utils_messenger(debug_utils_messenger, None);
                    instance.destroy_instance(None);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
