use std::str::FromStr;

use imnodes::{InputPinId, NodeId};

use crate::{
    exprtree::{ExpressionNode, Sign},
    pins::{InputPin, Pin},
    utils::ModelFragment, core::app::AppState,
};

use super::{
    ExprWrapper, LinkEvent, PendingOperation, PendingOperations, NodeImpl,
};

#[derive(Debug)]
pub struct Assigner {
    pub id: NodeId,
    pub name: String,
    pub input: InputPin,
    pub expr_node: ExprWrapper<Option<ExpressionNode<InputPinId>>>,
    pub operates_on: Option<(NodeId, String)>,
}

impl NodeImpl for Assigner {
    fn id(&self) -> imnodes::NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn send_data(&self) -> ExpressionNode<InputPinId> {
        unreachable!("This node doesn't feature an output pin")
    }

    fn on_link_event(&mut self, link_event: LinkEvent) -> bool {
        match link_event {
            LinkEvent::Push { payload, .. } => {
                let payload = self.input.map_data(payload);
                self.expr_node.set_expr(Some(payload))
            }
            LinkEvent::Pop(_) => self.expr_node.set_expr(None),
        };

        false
    }

    fn state_changed(&mut self) -> bool {
        if let Some(ref mut expr_node) = *self.expr_node {
            expr_node.set_unary(self.input.sign);
            self.expr_node.resolution.reset();
        }

        false
    }

    fn draw(&mut self, ui: &imgui::Ui) -> bool {
        match self.expr_node.get_expr_repr() {
            Some(expr) => ui.text(expr),
            None => ui.text("Nothing yet!"),
        }

        ui.text("Variable: ");
        ui.same_line();
        
        match &self.operates_on {
            Some((_, node_name)) => {
                ui.text(node_name);
                ui.button("Change")
            },
            None => ui.button("Choose"),
        }
    }

    fn trigger_app_state_change(&self) -> Option<AppState> {
        Some(
            AppState::AttributingAssignerOperatesOn {
                attribute_to: self.id,
            }
        )
    }

    fn inputs(&self) -> Option<&[InputPin]> {
        Some(std::array::from_ref(&self.input))
    }

    fn inputs_mut(&mut self) -> Option<&mut [InputPin]> {
        Some(std::array::from_mut(&mut self.input))
    }

    fn to_model_fragment(&self, app: &crate::core::App) -> Option<ModelFragment> {
        let Some(linked_pin_id) = self.input.linked_to else {
            return None;
        };

        let node_id = app
            .output_pins
            .get(&linked_pin_id)
            .expect("The node must exist, otherwise this should have been unlinked");

        let node = app
            .get_node(*node_id)
            .expect("The node must exist, otherwise this should have been unlinked");

        let argument = node.name().to_owned();
        let contribution = self.input.sign.into();

        Some(odeir::Equation {
            name: self.name().to_owned(),
            operates_on: "TODO!".to_string(),
            argument,
            contribution,
        }.into())
    }
    fn new(node_id: NodeId, name: String) -> Self {
        Self {
            id: node_id,
            name,
            input: InputPin::new_signed(node_id, Sign::Positive),
            expr_node: Default::default(),
            operates_on: None,
        }
    }

    fn try_from_model_fragment(
        node_id: NodeId,
        frag: &ModelFragment,
    ) -> Option<(Self, Option<PendingOperations>)> {
        let ModelFragment::Equation(eq) = frag else {
            return None;
        };

        let mut tmp = [0; 4];
        let contribution = eq.contribution.encode_utf8(&mut tmp);

        let node = Self {
            id: node_id,
            name: eq.name.clone(),
            input: InputPin::new_signed(
                node_id,
                Sign::from_str(contribution).expect("Should be a valid sign"),
            ),
            expr_node: Default::default(),
            operates_on: None,
        };

        let pending_ops = PendingOperations {
            node_id,
            operations: vec![PendingOperation::LinkWith {
                node_name: eq.argument.clone(),
                via_pin_id: *node.input.id(),
                sign: node.input.sign,
            }],
        };

        Some((node, Some(pending_ops)))
    }
}