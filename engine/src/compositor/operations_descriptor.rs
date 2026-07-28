#[derive(Clone, Debug)]
pub struct OperationDescriptor {
    pub id: &'static str,
    pub menu: &'static str,
    pub label: &'static str,
    pub action: &'static str,
}