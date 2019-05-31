use crate::cx::*;
use time::precise_time_ns;
use std:: {ptr};
use winapi::um:: {libloaderapi, winuser};
use winapi::shared::minwindef:: {LPARAM, LRESULT, WPARAM, BOOL, UINT, FALSE};
use winapi::um::winnt::{LPCWSTR};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::mem;
use std::os::raw::c_void;
use std::sync:: {Once, ONCE_INIT};
use winapi::shared::windef:: {RECT, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE, HMONITOR, HWND,};
use winapi::shared::winerror::S_OK;
use winapi::um::libloaderapi:: {GetProcAddress, LoadLibraryA};
use winapi::um::shellscalingapi:: {MDT_EFFECTIVE_DPI, MONITOR_DPI_TYPE, PROCESS_DPI_AWARENESS, PROCESS_PER_MONITOR_DPI_AWARE,};
use winapi::um::wingdi:: {GetDeviceCaps, LOGPIXELSX};
use winapi::um::winnt:: {HRESULT, LPCSTR};
use winapi::um::winuser:: {MONITOR_DEFAULTTONEAREST};


#[derive(Default)]
pub struct WindowsWindow {
    pub last_window_geom: WindowGeom,
    
    pub time_start: u64,
    pub last_key_mod: KeyModifiers,
    pub ime_spot: Vec2,
    
    pub current_cursor: MouseCursor,
    pub last_mouse_pos: Vec2,
    pub fingers_down: Vec<bool>,
    pub hwnd: Option<HWND>,
    pub event_callback: Option<*mut FnMut(&mut Vec<Event>)>,
    pub dpi_functions: Option<DpiFunctions>
}

impl WindowsWindow {
    
    pub unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> LRESULT {
        
        let user_data = winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA);
        if user_data == 0 {
            return winuser::DefWindowProcW(hwnd, msg, wparam, lparam);
        };
        
        let window = &(*(user_data as *mut WindowsWindow));
        match msg {
            winuser::WM_MOUSEMOVE => {
                window.on_mouse_move();
            },
            _ => ()
        }
        // lets get the window
        // Unwinding into foreign code is undefined behavior. So we catch any panics that occur in our
        // code, and if a panic happens we cancel any future operations.
        //run_catch_panic(-1, || callback_inner(window, msg, wparam, lparam))
        return winuser::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
    
    pub fn on_mouse_move(&self) {
    }
    
    pub fn init(&mut self, title: &str) {
        self.time_start = precise_time_ns();
        for _i in 0..10 {
            self.fingers_down.push(false);
        }
        
        self.dpi_functions = Some(DpiFunctions::new());
        if let Some(dpi_functions) = &self.dpi_functions {
            dpi_functions.become_dpi_aware()
        }
        
        let class_name_wstr: Vec<_> = OsStr::new("MakepadWindow").encode_wide().chain(Some(0).into_iter()).collect();
        
        let class = winuser::WNDCLASSEXW {
            cbSize: mem::size_of::<winuser::WNDCLASSEXW>() as UINT,
            style: winuser::CS_HREDRAW | winuser::CS_VREDRAW | winuser::CS_OWNDC,
            lpfnWndProc: Some(WindowsWindow::window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: unsafe {libloaderapi::GetModuleHandleW(ptr::null())},
            hIcon: unsafe {winuser::LoadIconW(ptr::null_mut(), winuser::IDI_WINLOGO)}, //h_icon,
            hCursor: unsafe {winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_ARROW)}, // must be null in order for cursor state to work properly
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name_wstr.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        
        unsafe {winuser::RegisterClassExW(&class);}
        
        let style = winuser::WS_SIZEBOX | winuser::WS_MAXIMIZEBOX | winuser::WS_CAPTION
            | winuser::WS_MINIMIZEBOX | winuser::WS_BORDER | winuser::WS_VISIBLE
            | winuser::WS_CLIPSIBLINGS | winuser::WS_CLIPCHILDREN | winuser::WS_SYSMENU;
        
        let style_ex = winuser::WS_EX_WINDOWEDGE | winuser::WS_EX_APPWINDOW | winuser::WS_EX_ACCEPTFILES;
        
        unsafe {
            // lets store the window
            winuser::IsGUIThread(1);
            
            let title_wstr: Vec<_> = OsStr::new(title).encode_wide().chain(Some(0).into_iter()).collect();
            
            let hwnd = winuser::CreateWindowExW(
                style_ex,
                class_name_wstr.as_ptr(),
                title_wstr.as_ptr() as LPCWSTR,
                style,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                ptr::null_mut(),
                ptr::null_mut(),
                libloaderapi::GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            self.hwnd = Some(hwnd);
            winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, &self as *const _ as isize);
            
            if let Some(dpi_functions) = &self.dpi_functions {
                dpi_functions.enable_non_client_dpi_scaling(self.hwnd.unwrap())
            }
        }
    }
    
    pub fn poll_events<F>(&mut self, first_block: bool, mut event_handler: F)
    where F: FnMut(&mut Vec<Event>),
    {
        unsafe {
            self.event_callback = Some(&mut event_handler as *const FnMut(&mut Vec<Event>) as *mut FnMut(&mut Vec<Event>));
            let mut msg = mem::uninitialized();
            loop {
                if first_block {
                    if winuser::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) == 0 {
                        // Only happens if the message is `WM_QUIT`.
                        debug_assert_eq!(msg.message, winuser::WM_QUIT);
                        break;
                    }
                }
                else {
                    if winuser::PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, 1) == 0 {
                        break;
                    }
                }
                // Calls `callback` below.
                winuser::TranslateMessage(&msg);
                winuser::DispatchMessageW(&msg);
            }
            self.event_callback = None;
        }
    }
    
    pub fn do_callback(&mut self, events: &mut Vec<Event>) {
        unsafe {
            if self.event_callback.is_none() {
                return
            };
            let callback = self.event_callback.unwrap();
            (*callback)(events);
        }
    }
    
    pub fn set_mouse_cursor(&mut self, _cursor: MouseCursor) {
    }
    
    pub fn get_window_geom(&self) -> WindowGeom {
        WindowGeom {
            inner_size: self.get_inner_size(),
            outer_size: self.get_outer_size(),
            dpi_factor: self.get_dpi_factor(),
            position: self.get_position()
        }
    }
    
    pub fn time_now(&self) -> f64 {
        let time_now = precise_time_ns();
        (time_now - self.time_start) as f64 / 1_000_000_000.0
    }
    
    pub fn set_position(&mut self, _pos: Vec2) {
    }
    
    pub fn get_position(&self) -> Vec2 {
        unsafe {
            let mut rect = RECT {left: 0, top: 0, bottom: 0, right: 0};
            winuser::GetWindowRect(self.hwnd.unwrap(), &mut rect);
            Vec2 {x: rect.left as f32, y: rect.top as f32}
        }
    }
    
    fn get_ime_origin(&self) -> Vec2 {
        Vec2::zero()
    }
    
    pub fn get_inner_size(&self) -> Vec2 {
        unsafe {
            let mut rect = RECT {left: 0, top: 0, bottom: 0, right: 0};
            winuser::GetClientRect(self.hwnd.unwrap(), &mut rect);
            Vec2 {x: (rect.right - rect.left) as f32, y: (rect.bottom - rect.top)as f32}
        }
    }
    
    pub fn get_outer_size(&self) -> Vec2 {
        unsafe {
            let mut rect = RECT {left: 0, top: 0, bottom: 0, right: 0};
            winuser::GetWindowRect(self.hwnd.unwrap(), &mut rect);
            Vec2 {x: (rect.right - rect.left) as f32, y: (rect.bottom - rect.top)as f32}
        }
    }
    
    pub fn set_outer_size(&self, _size: Vec2) {
    }
    
    pub fn get_dpi_factor(&self) -> f32 {
        if let Some(dpi_functions) = &self.dpi_functions {
            dpi_functions.hwnd_dpi_factor(self.hwnd.unwrap())
        }
        else {
            1.0
        }
    }
    
    pub fn start_timer(&mut self, _timer_id: u64, _interval: f64, _repeats: bool) {
    }
    
    pub fn stop_timer(&mut self, _timer_id: u64) {
    }
    
    pub fn post_signal(_signal_id: u64, _value: u64) {
    }
    
    pub fn send_change_event(&mut self) {
        
        let new_geom = self.get_window_geom();
        let old_geom = self.last_window_geom.clone();
        self.last_window_geom = new_geom.clone();
        
        self.do_callback(&mut vec![Event::WindowChange(WindowChangeEvent {
            old_geom: old_geom,
            new_geom: new_geom
        })]);
    }
    
    pub fn send_focus_event(&mut self) {
        self.do_callback(&mut vec![Event::AppFocus]);
    }
    
    pub fn send_focus_lost_event(&mut self) {
        self.do_callback(&mut vec![Event::AppFocusLost]);
    }
    
    pub fn send_finger_down(&mut self, digit: usize, modifiers: KeyModifiers) {
        self.fingers_down[digit] = true;
        self.do_callback(&mut vec![Event::FingerDown(FingerDownEvent {
            abs: self.last_mouse_pos,
            rel: self.last_mouse_pos,
            rect: Rect::zero(),
            digit: digit,
            handled: false,
            is_touch: false,
            modifiers: modifiers,
            tap_count: 0,
            time: self.time_now()
        })]);
    }
    
    pub fn send_finger_up(&mut self, digit: usize, modifiers: KeyModifiers) {
        self.fingers_down[digit] = false;
        self.do_callback(&mut vec![Event::FingerUp(FingerUpEvent {
            abs: self.last_mouse_pos,
            rel: self.last_mouse_pos,
            rect: Rect::zero(),
            abs_start: Vec2::zero(),
            rel_start: Vec2::zero(),
            digit: digit,
            is_over: false,
            is_touch: false,
            modifiers: modifiers,
            time: self.time_now()
        })]);
    }
    
    pub fn send_finger_hover_and_move(&mut self, pos: Vec2, modifiers: KeyModifiers) {
        self.last_mouse_pos = pos;
        let mut events = Vec::new();
        for (digit, down) in self.fingers_down.iter().enumerate() {
            if *down {
                events.push(Event::FingerMove(FingerMoveEvent {
                    abs: pos,
                    rel: pos,
                    rect: Rect::zero(),
                    digit: digit,
                    abs_start: Vec2::zero(),
                    rel_start: Vec2::zero(),
                    is_over: false,
                    is_touch: false,
                    modifiers: modifiers.clone(),
                    time: self.time_now()
                }));
            }
        };
        events.push(Event::FingerHover(FingerHoverEvent {
            abs: pos,
            rel: pos,
            rect: Rect::zero(),
            handled: false,
            hover_state: HoverState::Over,
            modifiers: modifiers,
            time: self.time_now()
        }));
        self.do_callback(&mut events);
    }
    
    pub fn send_close_requested_event(&mut self) {
        self.do_callback(&mut vec![Event::CloseRequested])
    }
    
    pub fn send_text_input(&mut self, input: String, replace_last: bool) {
        self.do_callback(&mut vec![Event::TextInput(TextInputEvent {
            input: input,
            was_paste: false,
            replace_last: replace_last
        })])
    }
}

// reworked from winit windows platform https://github.com/rust-windowing/winit/blob/eventloop-2.0/src/platform_impl/windows/dpi.rs

const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: DPI_AWARENESS_CONTEXT = -4isize as _;
type SetProcessDPIAware = unsafe extern "system" fn () -> BOOL;
type SetProcessDpiAwareness = unsafe extern "system" fn (value: PROCESS_DPI_AWARENESS,) -> HRESULT;
type SetProcessDpiAwarenessContext = unsafe extern "system" fn (value: DPI_AWARENESS_CONTEXT,) -> BOOL;
type GetDpiForWindow = unsafe extern "system" fn (hwnd: HWND) -> UINT;
type GetDpiForMonitor = unsafe extern "system" fn (hmonitor: HMONITOR, dpi_type: MONITOR_DPI_TYPE, dpi_x: *mut UINT, dpi_y: *mut UINT,) -> HRESULT;
type EnableNonClientDpiScaling = unsafe extern "system" fn (hwnd: HWND) -> BOOL;

// Helper function to dynamically load function pointer.
// `library` and `function` must be zero-terminated.
fn get_function_impl(library: &str, function: &str) -> Option<*const c_void> {
    // Library names we will use are ASCII so we can use the A version to avoid string conversion.
    let module = unsafe {LoadLibraryA(library.as_ptr() as LPCSTR)};
    if module.is_null() {
        return None;
    }
    
    let function_ptr = unsafe {GetProcAddress(module, function.as_ptr() as LPCSTR)};
    if function_ptr.is_null() {
        return None;
    }
    
    Some(function_ptr as _)
}

macro_rules!get_function {
    ( $ lib: expr, $ func: ident) => {
        get_function_impl(concat!( $ lib, '\0'), concat!(stringify!( $ func), '\0'))
            .map( | f | unsafe {mem::transmute::<*const _, $ func>(f)})
    }
}

pub struct DpiFunctions {
    get_dpi_for_window: Option<GetDpiForWindow>,
    get_dpi_for_monitor: Option<GetDpiForMonitor>,
    enable_nonclient_dpi_scaling: Option<EnableNonClientDpiScaling>,
    set_process_dpi_awareness_context: Option<SetProcessDpiAwarenessContext>,
    set_process_dpi_awareness: Option<SetProcessDpiAwareness>,
    set_process_dpi_aware: Option<SetProcessDPIAware>,
}

const BASE_DPI: u32 = 96;

impl DpiFunctions {
    fn new() -> DpiFunctions {
        DpiFunctions {
            get_dpi_for_window: get_function!("user32.dll", GetDpiForWindow),
            get_dpi_for_monitor: get_function!("shcore.dll", GetDpiForMonitor),
            enable_nonclient_dpi_scaling: get_function!("user32.dll", EnableNonClientDpiScaling),
            set_process_dpi_awareness_context: get_function!("user32.dll", SetProcessDpiAwarenessContext),
            set_process_dpi_awareness: get_function!("shcore.dll", SetProcessDpiAwareness),
            set_process_dpi_aware: get_function!("user32.dll", SetProcessDPIAware)
        }
    }
    
    fn become_dpi_aware(&self) {
        unsafe {
            if let Some(set_process_dpi_awareness_context) = self.set_process_dpi_awareness_context {
                // We are on Windows 10 Anniversary Update (1607) or later.
                if set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == FALSE {
                    // V2 only works with Windows 10 Creators Update (1703). Try using the older
                    // V1 if we can't set V2.
                    set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
                }
            }
            else if let Some(set_process_dpi_awareness) = self.set_process_dpi_awareness {
                // We are on Windows 8.1 or later.
                set_process_dpi_awareness(PROCESS_PER_MONITOR_DPI_AWARE);
            }
            else if let Some(set_process_dpi_aware) = self.set_process_dpi_aware {
                // We are on Vista or later.
                set_process_dpi_aware();
            }
        }
    }
    
    pub fn enable_non_client_dpi_scaling(&self, hwnd: HWND) {
        unsafe {
            if let Some(enable_nonclient_dpi_scaling) = self.enable_nonclient_dpi_scaling {
                enable_nonclient_dpi_scaling(hwnd);
            }
        }
    }
    /*
    pub fn get_monitor_dpi(hmonitor: HMONITOR) -> Option<u32> {
        unsafe {
            if let Some(GetDpiForMonitor) = *GET_DPI_FOR_MONITOR {
                // We are on Windows 8.1 or later.
                let mut dpi_x = 0;
                let mut dpi_y = 0;
                if GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == S_OK {
                    // MSDN says that "the values of *dpiX and *dpiY are identical. You only need to
                    // record one of the values to determine the DPI and respond appropriately".
                    // https://msdn.microsoft.com/en-us/library/windows/desktop/dn280510(v=vs.85).aspx
                    return Some(dpi_x as u32)
                }
            }
        }
        None
    }*/
    
    pub fn hwnd_dpi_factor(&self, hwnd: HWND) -> f32 {
        unsafe {
            let hdc = winuser::GetDC(hwnd);
            if hdc.is_null() {
                panic!("`GetDC` returned null!");
            }
            let dpi = if let Some(get_dpi_for_window) = self.get_dpi_for_window {
                // We are on Windows 10 Anniversary Update (1607) or later.
                match get_dpi_for_window(hwnd) {
                    0 => BASE_DPI, // 0 is returned if hwnd is invalid
                    dpi => dpi as u32,
                }
            }
            else if let Some(get_dpi_for_monitor) = self.get_dpi_for_monitor {
                // We are on Windows 8.1 or later.
                let monitor = winuser::MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                if monitor.is_null() {
                    BASE_DPI
                }
                else {
                    let mut dpi_x = 0;
                    let mut dpi_y = 0;
                    if get_dpi_for_monitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == S_OK {
                        dpi_x as u32
                    } else {
                        BASE_DPI
                    }
                }
            }
            else {
                // We are on Vista or later.
                if winuser::IsProcessDPIAware() != FALSE {
                    // If the process is DPI aware, then scaling must be handled by the application using
                    // this DPI value.
                    GetDeviceCaps(hdc, LOGPIXELSX) as u32
                } else {
                    // If the process is DPI unaware, then scaling is performed by the OS; we thus return
                    // 96 (scale factor 1.0) to prevent the window from being re-scaled by both the
                    // application and the WM.
                    BASE_DPI
                }
            };
            dpi as f32 / BASE_DPI as f32
        }
    }
    
}