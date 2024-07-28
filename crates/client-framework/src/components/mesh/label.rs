/// 정점 속성의 식별자 입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VertexAttributeLabel {
    Color, 
    Position, 
    Texcoord0, 
    Texcoord1, 
    Normal, 
    Tangent, 
    Joint, 
    Weight, 
}
