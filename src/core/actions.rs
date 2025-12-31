pub enum Action {
    // Movement
    Left,
    Right,
    Up,
    Down,
    Top,
    Bottom,
    MaxLeft,
    MaxRight,
    
    // Movement with selection (Shift+arrows)
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectTop,
    SelectBottom,
    SelectMaxLeft,
    SelectMaxRight,
    
    // Mouse
    MouseDown(u16, u16),
    MouseDrag(u16, u16),
    MouseUp(u16, u16),
    MouseDoubleClick(u16, u16),
    MouseTripleClick(u16, u16),
    
    // Text operations
    NextLine,
    Backspace,
    Delete,
    ToggleCtrlShortcuts,
    Save,
    New,
    Quit,
    Print,
}