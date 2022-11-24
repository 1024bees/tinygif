/// Known GIF block labels.
///
/// Note that the block uniquely specifies the layout of bytes that follow and how they are
/// framed. For example, the header always has a fixed length but is followed by a variable amount
/// of additional data. An image descriptor may be followed by a local color table depending on
/// information read in it. Therefore, it doesn't make sense to continue parsing after encountering
/// an unknown block as the semantics of following bytes are unclear.
///
/// The extension block provides a common framing for an arbitrary amount of application specific
/// data which may be ignored.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Block {
    /// Image block.
    Image = 0x2C,
    /// Extension block.
    Extension = 0x21,
    /// Image trailer.
    Trailer = 0x3B,
}

/// Known GIF Extension labels.
///
/// Note that we only support the graphics control extension; we include the other extensions so
/// that we can ignore them :)
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ExtensionLabel {
    /// Image block.
    Graphics = 0xf9,
    /// Extension block.
    App = 0xff,
    /// Text extension.
    Text = 0x01,
    /// Comment extension
    Comment = 0xfe,
}

impl ExtensionLabel {
    pub fn from_u8(n: u8) -> Result<Self, ParseError> {
        match n {
            0xf9 => Ok(ExtensionLabel::Graphics),
            0xff => Ok(ExtensionLabel::App),
            0x01 => Ok(ExtensionLabel::Text),
            0xfe => Ok(ExtensionLabel::Comment),
            _ => Err(ParseError::IncorrectExtension),
        }
    }
}

impl Block {
    /// Try to parse from u8
    pub fn from_u8(n: u8) -> Result<Self, ParseError> {
        match n {
            0x2C => Ok(Block::Image),
            0x21 => Ok(Block::Extension),
            0x3B => Ok(Block::Trailer),
            _ => Err(ParseError::IncorrectBlockLabel),
        }
    }
}

/// Errors that emerge when parsing our gif file
#[derive(Debug)]
pub enum ParseError {
    ///Malformed GIF file
    BadGifFile,
    ///EoF came early
    UnepectedEOF,
    ///Invalid block label
    IncorrectBlockLabel,
    ///Invalid extension label
    IncorrectExtension,
    ///Seeking to offset failed. TODO: We should remove this error in the future when we allow for
    ///rewindable iterators
    SeekFail,
    ///No images left
    NoImagesLeft
}
