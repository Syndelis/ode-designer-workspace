pub mod combinator;
pub mod constant;
pub mod population;

pub use combinator::Combinator;
pub use constant::Constant;
pub use population::Population;

use imgui::Ui;
use imnodes::{InputPinId, NodeId, NodeScope, OutputPinId};
use linkme::distributed_slice;

use crate::{
    imgui::app::sign_pin_button,
    message::{Message, SendData},
    pins::{InputClass, InputPin, OutputPin, PinClass},
};

use super::Data;

pub type NameAndConstructor = (&'static str, fn(NodeId) -> Box<dyn NodeSpecialization>);

#[distributed_slice]
pub static NODE_SPECIALIZATIONS: [NameAndConstructor] = [..];

pub enum ParentContext<'dc> {
    String(&'dc str),
}

pub trait NodeSpecialization: std::fmt::Debug {
    fn on_data_received(&mut self, from_pin_id: InputPinId, data: Data) -> bool {
        false
    }
    fn send_data(&self, ctx: &[ParentContext]) -> Data;

    fn draw(&mut self, ui: &Ui, ctx: &[ParentContext]) -> bool;

    fn inputs(&self) -> Option<&[InputPin]> {
        None
    }
    fn outputs(&self) -> Option<&[OutputPin]> {
        None
    }

    fn inputs_mut(&mut self) -> Option<&mut [InputPin]> {
        None
    }
    fn outputs_mut(&mut self) -> Option<&mut [OutputPin]> {
        None
    }

    fn broadcast_data(&self, ctx: &[ParentContext]) -> Vec<Message> {
        let data = self.send_data(ctx);
        self.outputs()
            .expect("Tried broadcasting data to node without any output pins")
            .iter()
            .flat_map(|output| {
                output.linked_to.iter().copied().map(|to_input| SendData {
                    data: data.clone(),
                    from_output: output.id,
                    to_input,
                })
            })
            .map(Message::from)
            .collect()
    }

    fn receive_data(
        &mut self,
        from_pin_id: InputPinId,
        data: Data,
        ctx: &[ParentContext],
    ) -> Option<Vec<Message>> {
        if self.on_data_received(from_pin_id, data) {
            Some(self.broadcast_data(ctx))
        } else {
            None
        }
    }

    fn process_node(
        &mut self,
        ui: &Ui,
        ui_node: &mut NodeScope,
        ctx: &[ParentContext],
    ) -> Option<Vec<Message>> {
        ui_node.add_titlebar(|| ui.text("TODO"));

        let mut input_changed = false;

        for input in self.inputs_mut().unwrap_or_default() {
            let shape = input.get_shape();
            let id = *input.id();
            match &mut input.class {
                PinClass::Input(input_class) => {
                    ui_node.add_input(id, shape, || match input_class {
                        InputClass::Signed(sign) => {
                            if sign_pin_button(ui, id.into(), sign) {
                                sign.toggle();
                                input_changed = true;
                            }
                        }
                        InputClass::Normal => {}
                    })
                }
                PinClass::Output => {
                    unreachable!("we're iterating over the inputs list, noone can be an output")
                }
            }
        }

        for output in self.outputs_mut().unwrap_or_default() {
            let shape = output.get_shape();
            let id = *output.id();
            match &mut output.class {
                PinClass::Input(_) => {
                    unreachable!("we're iterating over the outputs list, noone can be an input")
                }
                PinClass::Output => ui_node.add_output(id, shape, || {}),
            }
        }

        let inner_content_changed = self.draw(ui, ctx);

        if inner_content_changed || input_changed {
            Some(self.broadcast_data(ctx))
        } else {
            None
        }
    }

    fn get_input(&self, input_pin_id: &InputPinId) -> Option<&InputPin> {
        self.inputs()
            .unwrap_or_default()
            .iter()
            .find(|pin| pin.id() == input_pin_id)
    }

    fn get_output(&self, output_pin_id: &OutputPinId) -> Option<&OutputPin> {
        self.outputs()
            .unwrap_or_default()
            .iter()
            .find(|pin| pin.id() == output_pin_id)
    }

    fn get_input_mut(&mut self, input_pin_id: &InputPinId) -> Option<&mut InputPin> {
        self.inputs_mut()
            .unwrap_or_default()
            .iter_mut()
            .find(|pin| pin.id() == input_pin_id)
    }

    fn get_output_mut(&mut self, output_pin_id: &OutputPinId) -> Option<&mut OutputPin> {
        self.outputs_mut()
            .unwrap_or_default()
            .iter_mut()
            .find(|pin| pin.id() == output_pin_id)
    }
}

pub trait NodeSpecializationInitializer {
    fn new(node_id: NodeId) -> Self;

    fn new_boxed(node_id: NodeId) -> Box<dyn NodeSpecialization>
    where
        Self: NodeSpecialization + Sized + 'static,
    {
        Box::new(Self::new(node_id))
    }
}
