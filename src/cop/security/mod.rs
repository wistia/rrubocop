mod eval;
mod io_methods;
mod json_load;
mod open;
mod yaml_load;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(eval::Eval));
    registry.register(Box::new(io_methods::IoMethods));
    registry.register(Box::new(json_load::JsonLoad));
    registry.register(Box::new(open::Open));
    registry.register(Box::new(yaml_load::YamlLoad));
}
