//! The toolbar: which tools there are, their keys and the one-line hint each shows.

/// A tool in SolidSketch's toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Line,
    Rectangle,
    Circle,
    PushPull,
    Move,
    Rotate,
    Scale,
    FollowMe,
    Paint,
    Tape,
    Orbit,
}

impl Tool {
    /// Toolbar order, top to bottom.
    pub const ALL: [Tool; 12] = [
        Tool::Select,
        Tool::Line,
        Tool::Rectangle,
        Tool::Circle,
        Tool::PushPull,
        Tool::Move,
        Tool::Rotate,
        Tool::Scale,
        Tool::FollowMe,
        Tool::Paint,
        Tool::Tape,
        Tool::Orbit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Line => "Line",
            Tool::Rectangle => "Rectangle",
            Tool::Circle => "Circle",
            Tool::PushPull => "Push/Pull",
            Tool::Move => "Move",
            Tool::Rotate => "Rotate",
            Tool::Scale => "Scale",
            Tool::FollowMe => "Follow Me",
            Tool::Paint => "Paint",
            Tool::Tape => "Tape Measure",
            Tool::Orbit => "Orbit",
        }
    }

    /// The key that picks the tool. Space is Select.
    pub fn key(self) -> char {
        match self {
            Tool::Select => ' ',
            Tool::Line => 'L',
            Tool::Rectangle => 'R',
            Tool::Circle => 'C',
            Tool::PushPull => 'P',
            Tool::Move => 'M',
            Tool::Rotate => 'Q',
            Tool::Scale => 'S',
            Tool::FollowMe => 'F',
            Tool::Paint => 'B',
            Tool::Tape => 'T',
            Tool::Orbit => 'O',
        }
    }

    /// What to do next, in a few words. The status bar shows it.
    pub fn hint(self) -> &'static str {
        match self {
            Tool::Select => "Click a face, edge or object. Shift adds to the selection.",
            Tool::Line => "Click to start. Click again for each corner. Type a length and press Enter.",
            Tool::Rectangle => "Click one corner, then the opposite one. Type width, height and press Enter.",
            Tool::Circle => "Click the centre, then drag out the radius. Type a radius and press Enter.",
            Tool::PushPull => "Click a face, then pull it out or push it in. Type a distance and press Enter.",
            Tool::Move => "Click what to move, then click where it goes.",
            Tool::Rotate => "Click what to turn, set the pivot, then swing it.",
            Tool::Scale => "Click what to resize, then drag a handle.",
            Tool::FollowMe => "Pick a path, then click the shape to sweep along it.",
            Tool::Paint => "Pick a colour or material, then click faces to paint them.",
            Tool::Tape => "Click two points to measure the distance between them.",
            Tool::Orbit => "Drag to look around. Scroll to zoom. Hold Shift to pan.",
        }
    }

    /// The tool a pressed key picks, case-insensitively.
    pub fn from_key(c: char) -> Option<Tool> {
        let up = c.to_ascii_uppercase();
        Tool::ALL.into_iter().find(|t| t.key() == up)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_tool_has_its_own_key() {
        let keys: HashSet<char> = Tool::ALL.iter().map(|t| t.key()).collect();
        assert_eq!(keys.len(), Tool::ALL.len());
    }

    #[test]
    fn keys_find_their_tool_in_either_case() {
        for t in Tool::ALL {
            assert_eq!(Tool::from_key(t.key()), Some(t));
            assert_eq!(Tool::from_key(t.key().to_ascii_lowercase()), Some(t));
        }
        assert_eq!(Tool::from_key('z'), None);
    }

    #[test]
    fn every_tool_says_what_to_do() {
        for t in Tool::ALL {
            assert!(!t.label().is_empty());
            assert!(t.hint().ends_with('.'));
        }
    }
}
