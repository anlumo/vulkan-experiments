// #![allow(unused)]

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
            Swapchain,
        },
    },
};
#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::GetModuleHandleA;
use std::{
    ffi::{CString, CStr},
    ptr::null,
    collections::HashSet,
    os::raw::c_char,
};

mod queue_families;
use crate::queue_families::QueueFamilyIndices;
mod suitability;
use crate::suitability::{is_device_suitable, DEVICE_EXTENSIONS};
mod swap_chain_support;
use crate::swap_chain_support::SwapChainSupportDetails;

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

#[derive(Default)]
struct SelectedDevice {
    suitability: u32,
    device: vk::PhysicalDevice,
    name: String,
    indices: QueueFamilyIndices,
    swap_chain_support_details: SwapChainSupportDetails,
}

struct VulkanExperiment {
    instance: ash::Instance,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    surface: vk::SurfaceKHR,
    physical_device: SelectedDevice,
    device: Option<ash::Device>,
    swapchain: vk::SwapchainKHR,
    swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    graphics_pipeline: vk::Pipeline,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,

    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    compiler: shaderc::Compiler,

    // VULKAN EXTENSIONS
    debug_utils_ext: DebugUtils,
    surface_ext: Surface,
    win32_surface_ext: Win32Surface,
    swapchain_ext: Option<Swapchain>,
}

type VulkanResult<T> = Result<T, Box<dyn std::error::Error>>;

impl VulkanExperiment {
    pub fn new(entry: &Entry) -> VulkanResult<Self> {
        trace!("VulkanExperiment::new");
        let instance = Self::create_instance(&entry)?;
        Ok(VulkanExperiment {
            debug_utils_messenger: Default::default(),
            surface: Default::default(),
            physical_device: Default::default(),
            device: Default::default(),
            swapchain: Default::default(),
            swapchain_extent: Default::default(),
            swapchain_images: Default::default(),
            swapchain_image_views: Default::default(),
            pipeline_layout: Default::default(),
            render_pass: Default::default(),
            graphics_pipeline: Default::default(),
            swapchain_framebuffers: Default::default(),
            command_pool: Default::default(),
            command_buffers: Default::default(),
            image_available_semaphore: Default::default(),
            render_finished_semaphore: Default::default(),

            graphics_queue: Default::default(),
            present_queue: Default::default(),

            compiler: shaderc::Compiler::new().unwrap(),

            debug_utils_ext: DebugUtils::new(entry, &instance),
            surface_ext: Surface::new(entry, &instance),
            win32_surface_ext: Win32Surface::new(entry, &instance),
            swapchain_ext: Default::default(),
            instance,
        })
    }
    fn layer_names() -> Vec<CString> {
        vec![CString::new("VK_LAYER_LUNARG_standard_validation").unwrap()]
    }
    pub fn create_instance(entry: &Entry) -> VulkanResult<ash::Instance> {
        trace!("create_instance");
        let app_info = vk::ApplicationInfo {
            api_version: vk_make_version!(1, 0, 0),
            ..Default::default()
        };
        let layer_names = Self::layer_names();
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
            .iter().copied().collect();
        let mut debug_messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(debug_messenger_callback))
            .build();
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw)
            .push_next(&mut debug_messenger_info);

        Ok(unsafe { entry.create_instance(&create_info, None) }?)
    }
    pub fn setup_early_debug_logging(&mut self) -> VulkanResult<()> {
        trace!("setup_early_debug_logging");
        let debug_messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(debug_messenger_callback))
            .build();
        self.debug_utils_messenger = unsafe { self.debug_utils_ext.create_debug_utils_messenger(&debug_messenger_info, None) }?;
        Ok(())
    }
    pub unsafe fn setup_surface(&mut self, window: *mut std::os::raw::c_void) -> VulkanResult<()> {
        trace!("setup_surface");
        let surface_create_info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hinstance(GetModuleHandleA(null()) as *const std::ffi::c_void)
            .hwnd(window);

        self.surface = self.win32_surface_ext.create_win32_surface(&surface_create_info, None)?;
        Ok(())
    }
    pub fn select_physical_device(&mut self) -> VulkanResult<()> {
        trace!("select_physical_device");
        let physical_devices = unsafe { self.instance.enumerate_physical_devices() }?;
        let physical_device = physical_devices.into_iter().filter_map(|device| {
            match is_device_suitable(&self.instance, device, &self.surface_ext, self.surface) {
                Ok((suitability, name, swap_chain_support_details)) => {
                    if suitability > 0 {
                        let indices = QueueFamilyIndices::find(&self.instance, device, &self.surface_ext, self.surface);
                        if indices.is_device_suitable() {
                            return Some(SelectedDevice { suitability, device, name, indices, swap_chain_support_details });
                        }
                    }
                }
                Err(err) => {
                    error!("{}", err);
                }
            }
            None
        }).max_by_key(|val| val.suitability);
        if physical_device.is_none() {
            error!("No suitable graphics card found.");
            return Ok(());
        }
        self.physical_device = physical_device.unwrap();
        info!("Device selected: {}", self.physical_device.name);

        Ok(())
    }
    pub fn create_device(&mut self) -> VulkanResult<()> {
        trace!("create_device");
        let physical_device_features = vk::PhysicalDeviceFeatures::builder();
        let layer_names = Self::layer_names();
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let queue_families: HashSet<u32> = [
            self.physical_device.indices.graphics.unwrap(),
            self.physical_device.indices.present.unwrap(),
        ].iter().cloned().collect();
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = queue_families.into_iter().map(|queue_family| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family)
                .queue_priorities(&[1.0])
                .build()
        }).collect();
        let device_extensions: Vec<*const c_char> = DEVICE_EXTENSIONS.iter().map(|s| s.as_ptr()).collect();
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&physical_device_features)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&device_extensions);
        
        self.device = Some(unsafe { self.instance.create_device(self.physical_device.device, &device_create_info, None) }?);
        self.swapchain_ext = Some(Swapchain::new(&self.instance, self.device.as_ref().unwrap()));

        Ok(())
    }

    pub fn create_swapchain(&mut self, window: &winit::window::Window) -> VulkanResult<()> {
        trace!("create_swapchain");
        let surface_format = self.physical_device.swap_chain_support_details.choose_format();
        let present_mode = self.physical_device.swap_chain_support_details.choose_present_mode();
        self.swapchain_extent = self.physical_device.swap_chain_support_details.choose_swap_extent(window.inner_size().width as u32, window.inner_size().height as u32);
        let image_count = {
            if self.physical_device.swap_chain_support_details.capabilities.max_image_count > 0 &&
                    self.physical_device.swap_chain_support_details.capabilities.min_image_count + 1 > self.physical_device.swap_chain_support_details.capabilities.max_image_count {
                self.physical_device.swap_chain_support_details.capabilities.max_image_count
            } else {
                self.physical_device.swap_chain_support_details.capabilities.min_image_count + 1
            }
        };
        let mut swap_chain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(self.swapchain_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);
        
        let queue_family_indices = [self.physical_device.indices.graphics.unwrap(), self.physical_device.indices.present.unwrap()];

        swap_chain_create_info = if queue_family_indices[0] != queue_family_indices[1] {
            swap_chain_create_info.image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices)
        } else {
            swap_chain_create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(self.physical_device.swap_chain_support_details.capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
        };
        self.swapchain = unsafe { self.swapchain_ext.as_ref().unwrap().create_swapchain(&swap_chain_create_info, None) }?;
        self.swapchain_images = unsafe { self.swapchain_ext.as_ref().unwrap().get_swapchain_images(self.swapchain) }?;

        Ok(())
    }

    pub fn create_image_views(&mut self) -> VulkanResult<()> {
        trace!("create_image_views");
        let surface_format = self.physical_device.swap_chain_support_details.choose_format();
        self.swapchain_image_views = self.swapchain_images.iter().map(|image| {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .components(vk::ComponentMapping::builder().r(vk::ComponentSwizzle::IDENTITY).g(vk::ComponentSwizzle::IDENTITY).b(vk::ComponentSwizzle::IDENTITY).a(vk::ComponentSwizzle::IDENTITY).build())
                .subresource_range(vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build()
                );
            unsafe { self.device.as_ref().unwrap().create_image_view(&create_info, None) }
        }).collect::<Result<Vec<vk::ImageView>, ash::vk::Result>>()?;

        Ok(())
    }

    fn create_shader_module(&mut self, code: &str, filename: &str, kind: shaderc::ShaderKind) -> VulkanResult<vk::ShaderModule> {
        trace!("create_shader_module {}", filename);
        let artifact = self.compiler.compile_into_spirv(code, kind, filename, "main", None)?;
        let binary = artifact.as_binary();
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(binary);
        Ok(unsafe { self.device.as_ref().unwrap().create_shader_module(&create_info, None) }?)
    }

    pub fn create_graphics_pipeline(&mut self) -> VulkanResult<()> {
        trace!("create_graphics_pipeline");
        let vertex_shader   = self.create_shader_module(include_str!("shaders/triangle.vs"), "triangle.vs", shaderc::ShaderKind::Vertex)?;
        let fragment_shader = self.create_shader_module(include_str!("shaders/triangle.fs"), "triangle.fs", shaderc::ShaderKind::Fragment)?;

        let main = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader)
                .name(main)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader)
                .name(main)
                .build(),
        ];
        
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[])
            .vertex_attribute_descriptions(&[]);
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewports = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.swapchain_extent.width as f32)
            .height(self.swapchain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()
        ];
        let scissors = [vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(self.swapchain_extent)
            .build()
        ];
        
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .viewports(&viewports)
            .scissors(&scissors);
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);
        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            // .sample_mask(&[])
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);
        
        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build()
        ];

        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0; 4]);
        
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[])
            .push_constant_ranges(&[]);

        trace!("Creating pipeline layout");

        self.pipeline_layout = unsafe { self.device.as_ref().unwrap().create_pipeline_layout(&pipeline_layout_info, None) }?;

        let pipeline_infos = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(self.pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0)
            .base_pipeline_index(-1)
            .build()
        ];

        trace!("Creating graphics pipeline");

        self.graphics_pipeline = unsafe { self.device.as_ref().unwrap().create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None) }.map_err(|err| err.1)?[0];

        trace!("cleanup shader modules");

        unsafe {
            self.device.as_ref().unwrap().destroy_shader_module(vertex_shader, None);
            self.device.as_ref().unwrap().destroy_shader_module(fragment_shader, None);
        }
        Ok(())
    }

    pub fn create_render_pass(&mut self) -> VulkanResult<()> {
        trace!("create_render_pass");
        let color_attachments = [vk::AttachmentDescription::builder()
            .format(self.physical_device.swap_chain_support_details.choose_format().format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build()
        ];
        
        let color_attachment_refs = [vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()
        ];
        
        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs)
            .build()
        ];

        let dependencies = [vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build()
        ];
        
        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&color_attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        
        self.render_pass = unsafe { self.device.as_ref().unwrap().create_render_pass(&render_pass_info, None) }?;

        Ok(())
    }

    pub fn create_framebuffers(&mut self) -> VulkanResult<()> {
        trace!("create_framebuffers");
        self.swapchain_framebuffers = self.swapchain_image_views.iter().map(|image_views| {
            let attachments = [*image_views];

            let framebuffer_info = vk::FramebufferCreateInfo::builder()
                .render_pass(self.render_pass)
                .attachments(&attachments)
                .width(self.swapchain_extent.width)
                .height(self.swapchain_extent.height)
                .layers(1);
            unsafe { self.device.as_ref().unwrap().create_framebuffer(&framebuffer_info, None) }
        }).collect::<Result<Vec<_>,_>>()?;

        Ok(())
    }

    pub fn create_command_pool(&mut self) -> VulkanResult<()> {
        trace!("create_command_pool");
        let pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.physical_device.indices.graphics.unwrap());
        
        self.command_pool = unsafe { self.device.as_ref().unwrap().create_command_pool(&pool_info, None) }?;

        Ok(())
    }

    pub fn create_command_buffers(&mut self) -> VulkanResult<()> {
        trace!("create_command_buffers");
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(self.swapchain_framebuffers.len() as u32);
        
        let device = self.device.as_ref().unwrap();
        self.command_buffers = unsafe { device.allocate_command_buffers(&alloc_info) }?;

        for (command_buffer, framebuffer) in self.command_buffers.iter().zip(self.swapchain_framebuffers.iter()) {
            let begin_info = vk::CommandBufferBeginInfo::builder();
            unsafe { device.begin_command_buffer(*command_buffer, &begin_info) }?;

            let clear_color_values = [vk::ClearValue {
                color: vk::ClearColorValue::default(),
            }];
            let render_pass_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(*framebuffer)
                .render_area(vk::Rect2D::builder().offset(vk::Offset2D::builder().x(0).y(0).build()).extent(self.swapchain_extent).build())
                .clear_values(&clear_color_values);

            unsafe {
                device.cmd_begin_render_pass(*command_buffer, &render_pass_info, vk::SubpassContents::INLINE);
                device.cmd_bind_pipeline(*command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline);
                device.cmd_draw(*command_buffer, 3, 1, 0, 0);
                device.cmd_end_render_pass(*command_buffer);
            }
            unsafe { device.end_command_buffer(*command_buffer) }?;
        }

        Ok(())
    }

    pub fn create_semaphores(&mut self) -> VulkanResult<()> {
        trace!("create_semaphores");
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let device = self.device.as_ref().unwrap();
        self.image_available_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }?;
        self.render_finished_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }?;
        Ok(())
    }

    pub fn create_queues(&mut self) -> VulkanResult<()> {
        trace!("create_queues");
        self.graphics_queue = unsafe { self.device.as_ref().unwrap().get_device_queue(self.physical_device.indices.graphics.unwrap(), 0) };
        self.present_queue = unsafe { self.device.as_ref().unwrap().get_device_queue(self.physical_device.indices.present.unwrap(), 0) };
        Ok(())
    }

    pub fn draw_frame(&mut self) -> VulkanResult<()> {
        trace!("draw_frame");
        let (image_index, _) = unsafe { self.swapchain_ext.as_ref().unwrap().acquire_next_image(self.swapchain, std::u64::MAX, self.image_available_semaphore, vk::Fence::null()) }?;
        let image_indices = [
            image_index,
        ];

        let wait_semaphores = [
            self.image_available_semaphore,
        ];
        let wait_stages = [
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ];
        let signal_semaphores = [
            self.render_finished_semaphore,
        ];
        let submit_info = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&self.command_buffers[image_index as usize..])
            .signal_semaphores(&signal_semaphores).build()
        ];
        
        unsafe { self.device.as_ref().unwrap().queue_submit(self.graphics_queue, &submit_info, vk::Fence::null()) }?;

        let swapchains = [
            self.swapchain,
        ];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        if let Err(err) = unsafe { self.swapchain_ext.as_ref().unwrap().queue_present(self.present_queue, &present_info) } {
            error!("vkQueuePresentKHR: {}", err);
        }

        unsafe { self.device.as_ref().unwrap().device_wait_idle().unwrap() };
        Ok(())
    }
}

impl Drop for VulkanExperiment {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_ref().unwrap();
            device.device_wait_idle().unwrap();

            device.destroy_semaphore(self.render_finished_semaphore, None);
            device.destroy_semaphore(self.image_available_semaphore, None);

            device.destroy_command_pool(self.command_pool, None);
            for framebuffer in &self.swapchain_framebuffers {
                device.destroy_framebuffer(*framebuffer, None);
            }
            device.destroy_pipeline(self.graphics_pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_render_pass(self.render_pass, None);
            for image_view in &self.swapchain_image_views {
                device.destroy_image_view(*image_view, None);
            }
            self.swapchain_ext.as_ref().unwrap().destroy_swapchain(self.swapchain, None);
            self.surface_ext.destroy_surface(self.surface, None);
            device.destroy_device(None);
            self.debug_utils_ext.destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("log.yaml", Default::default())?;
    info!("Startup");

    let entry = Entry::new()?;
    let mut app = VulkanExperiment::new(&entry)?;
    app.setup_early_debug_logging()?;

    // *** WINDOW CREATION ***
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Experiment")
        .build(&event_loop).unwrap();

    unsafe { app.setup_surface(window.hwnd()) }?;
    app.select_physical_device()?;
    app.create_device()?;
    app.create_swapchain(&window)?;
    app.create_image_views()?;
    app.create_queues()?;
    app.create_render_pass()?;
    app.create_graphics_pipeline()?;
    app.create_framebuffers()?;
    app.create_command_pool()?;
    app.create_command_buffers()?;
    app.create_semaphores()?;

    let mut app = Some(app);

    // *** MAIN LOOP ***
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
                if let Some(mut inner_app) = app.take() {
                    inner_app.draw_frame().expect("Draw error");
                    app.replace(inner_app);
                }
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
                app.take();
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
