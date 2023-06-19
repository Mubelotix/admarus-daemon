pub use crate::{app::*, search::*, settings::*, util::*, results::*, result::*, api::*, *};
pub use js_sys::{Array, Function, Promise, Reflect::*};
pub use std::{time::Duration, rc::Rc};
pub use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
pub use wasm_bindgen_futures::{spawn_local, JsFuture};
pub use yew::{html::Scope, prelude::*};
pub use yew_template::template_html;
pub use web_sys::{window as old_window, *};
pub use serde::{Serialize, Deserialize, de::DeserializeOwned};

pub type AppLink = Scope<App>;
