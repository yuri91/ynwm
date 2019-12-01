/// Gets the offset of a field. Used by container_of!
macro_rules! offset_of(
    ($ty:ty, $field:ident) => {
        &(*(0 as *const $ty)).$field as *const _ as usize
    }
);

/// Gets the parent struct from a pointer.
/// VERY unsafe. The parent struct _must_ be repr(C), and the
/// type passed to this macro _must_ match the type of the parent.
macro_rules! container_of (
    ($ptr: expr, $container: ty, $field: ident) => {
        ($ptr as *mut u8).offset(-(offset_of!($container, $field) as isize)) as *mut $container
    }
);

/// Iterates over a wl_list.
///
/// # Safety
/// It is not safe to delete an element while iterating over the list,
/// so don't do it!
macro_rules! wl_list_for_each {
    ($ptr: expr, $field: ident, ($pos: ident : $container: ty) => $body: block) => {
        let mut $pos: *mut $container;
        $pos = container_of!($ptr.next, $container, $field);
        loop {
            if &(*$pos).$field as *const _ == &$ptr as *const _ {
                break;
            }
            {
                $body
            }
            $pos = container_of!((*$pos).$field.next, $container, $field);
        }
    };
}

/// Logs a message using wlroots' logging capability.
///
/// Example:
/// ```rust,no_run,ignore
/// #[macro_use]
/// use wlroots::log::{init_logging, L_DEBUG, L_ERROR};
///
/// // Call this once, at the beginning of your program.
/// init_logging(WLR_DEBUG, None);
///
/// wlr_log!(L_DEBUG, "Hello world");
/// wlr_log!(L_ERROR, "Could not {:#?} the {}", foo, bar);
/// ```
#[macro_export]
macro_rules! wlr_log {
    ($verb: expr, $($msg:tt)*) => {{
        /// Convert a literal string to a C string.
        /// Note: Does not check for internal nulls, nor does it do any conversions on
        /// the grapheme clustors. Just passes the bytes as is.
        /// So probably only works on ASCII.
        macro_rules! c_str {
            ($s:expr) => {
                concat!($s, "\0").as_ptr()
                    as *const libc::c_char
    }
        }
        use ::std::ffi::CString;
        unsafe {
            let fmt = CString::new(format!($($msg)*))
                .expect("Could not convert log message to C string");
            let raw = fmt.into_raw();
            _wlr_log($verb, c_str!("[%s:%lu] %s"),
                    c_str!(file!()), line!(), raw);
            // Deallocate string
            CString::from_raw(raw);
        }
    }}
}

macro_rules! connect_listener {
    ($ctx:expr, $manager:ident, $event:ident) => {{
        connect_listener!($ctx, $ctx.$manager, $manager, $event);
    }};
    ($ctx:expr, $manager:expr, $manager_ident: ident, $event:ident) => {{
        paste::expr! {
            $ctx.[<$manager_ident _ $event _listener>].notify = Some(Self::[<$manager_ident _ $event _listener_fn>]);
            wl_signal_add(&mut (*$manager).events.$event as *mut _ as _, &mut ctx.[<$manager_ident _ $event _listener>] as *mut _ as _);
        }
    }}
}

macro_rules! implement_listener {
    ($struct:ty, $manager:ident, $event:ident, $data_ty:ty) => {
        paste::item! {
            impl $struct {
                unsafe extern "C" fn [<$manager _ $event _listener_fn>](listener:
                                                            *mut wl_listener,
                                                            data: *mut libc::c_void) {
                    let s: &mut $struct = &mut (*container_of!(listener, $struct, [<$manager _ $event _listener>]));
                    let s = Pin::new_unchecked(s);
                    let d = data as *mut $data_ty;
                    s.[<$manager _ $event>](d);
                }
            }
        }
    }
}
