use glutin::dpi::*;
use glutin::GlContext;
use glutin::GlRequest;
use glutin::GlProfile;
use std::mem;
use std::ptr;
use std::ffi::CStr;

use crate::cx_shared::*;
use crate::cxturtle::*;
use crate::cxshaders::*;
use crate::cx_winit::*;
use crate::events::*;

impl Cx{
     pub fn exec_draw_list(&mut self, draw_list_id: usize){

        let draw_calls_len = self.drawing.draw_lists[draw_list_id].draw_calls_len;

        for draw_call_id in 0..draw_calls_len{
            let sub_list_id = self.drawing.draw_lists[draw_list_id].draw_calls[draw_call_id].sub_list_id;
            if sub_list_id != 0{
                self.exec_draw_list(sub_list_id);
            }
            else{
                let draw_list = &mut self.drawing.draw_lists[draw_list_id];
                let draw_call = &mut draw_list.draw_calls[draw_call_id];
                let sh = &self.shaders.shaders[draw_call.shader_id];
                let csh = &self.shaders.compiled_shaders[draw_call.shader_id];

                unsafe{
                    draw_call.resources.check_attached_vao(csh);

                    if draw_call.update_frame_id == self.drawing.frame_id{
                        // update the instance buffer data
                            gl::BindBuffer(gl::ARRAY_BUFFER, draw_call.resources.vb);
                            gl::BufferData(gl::ARRAY_BUFFER,
                                            (draw_call.instance.len() * mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                                            draw_call.instance.as_ptr() as *const _, gl::STATIC_DRAW);
                    }

                    gl::UseProgram(csh.program);
                    gl::BindVertexArray(draw_call.resources.vao);
                    let instances = draw_call.instance.len() / csh.instance_slots;
                    let indices = sh.geometry_indices.len();
                    CxShaders::set_uniform_buffer_fallback(&csh.uniforms_cx, &self.uniforms);
                    CxShaders::set_uniform_buffer_fallback(&csh.uniforms_dl, &draw_list.uniforms);
                    CxShaders::set_uniform_buffer_fallback(&csh.uniforms_dr, &draw_call.uniforms);
                    CxShaders::set_texture_slots(&csh.texture_slots, &draw_call.textures, &mut self.textures);
                    gl::DrawElementsInstanced(gl::TRIANGLES, indices as i32, gl::UNSIGNED_INT, ptr::null(), instances as i32);
                }
            }
        }
    }

    pub unsafe fn gl_string(raw_string: *const gl::types::GLubyte) -> String {
        if raw_string.is_null() { return "(NULL)".into() }
        String::from_utf8(CStr::from_ptr(raw_string as *const _).to_bytes().to_vec()).ok()
                                    .expect("gl_string: non-UTF8 string")
    }
    
  
    pub fn repaint(&mut self, glutin_window:&glutin::GlWindow){
        unsafe{
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
            gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
            gl::BlendFuncSeparate(gl::ONE, gl::ONE_MINUS_SRC_ALPHA, gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::BLEND);
            gl::ClearColor(self.clear_color.x, self.clear_color.y, self.clear_color.z, self.clear_color.w);
            gl::Clear(gl::COLOR_BUFFER_BIT|gl::DEPTH_BUFFER_BIT);
        }
        self.prepare_frame();        
        self.exec_draw_list(0);

        glutin_window.swap_buffers().unwrap();
    }

    fn resize_window_to_turtle(&mut self, glutin_window:&glutin::GlWindow){
        glutin_window.resize(PhysicalSize::new(
            (self.turtle.target_size.x * self.turtle.target_dpi_factor) as f64,
            (self.turtle.target_size.y * self.turtle.target_dpi_factor) as f64)
        );
    }
    
    pub fn event_loop<F>(&mut self, mut event_handler:F)
    where F: FnMut(&mut Cx, Event),
    { 
        let gl_request = GlRequest::Latest;
        let gl_profile = GlProfile::Core;

        let mut events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_title(format!("OpenGL - {}",self.title))
            .with_dimensions(LogicalSize::new(640.0, 480.0));
        let context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl(gl_request)
            .with_gl_profile(gl_profile);
        let glutin_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

        unsafe {
            glutin_window.make_current().unwrap();
            gl::load_with(|symbol| glutin_window.get_proc_address(symbol) as *const _);

            //let mut num_extensions = 0;
            //gl::GetIntegerv(gl::NUM_EXTENSIONS, &mut num_extensions);
            //let extensions: Vec<_> = (0 .. num_extensions).map(|num| {
            //   Cx::gl_string(gl::GetStringi(gl::EXTENSIONS, num as gl::types::GLuint))
            //}).collect();
            //println!("Extensions   : {}", extensions.join(", "))
        }

        // lets compile all shaders
        self.shaders.compile_all_ogl_shaders();

        self.load_binary_deps_from_file();

        while self.running{
            events_loop.poll_events(|winit_event|{
                let event = self.map_winit_event(winit_event, &glutin_window);
                if let Event::Resized(_) = &event{
                    self.resize_window_to_turtle(&glutin_window);
                    event_handler(self, event); 
                    self.redraw_area = Some(Area::zero());
                    self.redraw_none();
                    event_handler(self, Event::Redraw);
                    self.redraw_none();
                    self.repaint(&glutin_window);
                }
                else{
                    event_handler(self, event); 
                }
            });
            // call redraw event
            if let Some(_) = &self.redraw_dirty{
                self.redraw_area = self.redraw_dirty.clone();
                self.redraw_none();
                event_handler(self, Event::Redraw);
                self.paint_dirty = true;
            }
            // repaint everything if we need to
            if self.paint_dirty{
                self.paint_dirty = false;
                self.repaint(&glutin_window);
            }

            // wait for the next event
            if self.animations.len() == 0{
                events_loop.run_forever(|winit_event|{
                    let event = self.map_winit_event(winit_event, &glutin_window);
                    if let Event::Resized(_) = &event{
                        self.resize_window_to_turtle(&glutin_window);
                        event_handler(self, event); 
                        self.redraw_area = Some(Area::zero());
                        self.redraw_none();
                        event_handler(self, Event::Redraw);
                        self.repaint(&glutin_window);
                    }
                    else{
                        event_handler(self, event);
                    }
                    winit::ControlFlow::Break
                })
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct CxResources{
    pub winit:CxWinit
}

#[derive(Clone, Default)]
pub struct DrawListResources{
}


#[derive(Default,Clone)]
pub struct DrawCallResources{
    pub resource_shader_id:usize,
    pub vao:gl::types::GLuint,
    pub vb:gl::types::GLuint
}

impl DrawCallResources{

    pub fn check_attached_vao(&mut self, csh:&CompiledShader){
        if self.resource_shader_id != csh.shader_id{
            self.free();
        }
        // create the VAO
        unsafe{
            self.resource_shader_id = csh.shader_id;
            self.vao = mem::uninitialized();
            gl::GenVertexArrays(1, &mut self.vao);
            gl::BindVertexArray(self.vao);
            
            // bind the vertex and indexbuffers
            gl::BindBuffer(gl::ARRAY_BUFFER, csh.geom_vb);
            for attr in &csh.geom_attribs{
                gl::VertexAttribPointer(attr.loc, attr.size, gl::FLOAT, 0, attr.stride, attr.offset as *const () as *const _);
                gl::EnableVertexAttribArray(attr.loc);
            }

            // create and bind the instance buffer
            self.vb = mem::uninitialized();
            gl::GenBuffers(1, &mut self.vb);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vb);
            
            for attr in &csh.inst_attribs{
                gl::VertexAttribPointer(attr.loc, attr.size, gl::FLOAT, 0, attr.stride, attr.offset as *const () as *const _);
                gl::EnableVertexAttribArray(attr.loc);
                gl::VertexAttribDivisor(attr.loc, 1 as gl::types::GLuint);
            }

            // bind the indexbuffer
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, csh.geom_ib);
            gl::BindVertexArray(0);
        }
    }

    fn free(&mut self){
        unsafe{
            if self.vao != 0{
                gl::DeleteVertexArrays(1, &mut self.vao);
            }
            if self.vb != 0{
                gl::DeleteBuffers(1, &mut self.vb);
            }
        }
        self.vao = 0;
        self.vb = 0;
    }
}