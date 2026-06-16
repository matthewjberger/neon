//! The tour: a vimtutor-style lesson. It opens a throwaway buffer whose text
//! teaches the keys and asks you to practice on the buffer itself. You learn by
//! doing the motions and edits right here, not by reading tips.

use leptos::prelude::*;

use crate::state::{EditorState, FileBuffer, PluginKind};

const TOUR_PATH: &str = "neon-tour.txt";

/// Opens the tour buffer, replacing its text with a fresh lesson, and focuses it.
pub fn start(state: EditorState) {
    state.files.update(|files| {
        if let Some(file) = files.iter_mut().find(|file| file.path == TOUR_PATH) {
            file.text = LESSON.to_string();
            file.dirty = false;
        } else {
            files.push(FileBuffer {
                path: TOUR_PATH.to_string(),
                text: LESSON.to_string(),
                dirty: false,
            });
        }
    });
    state.open_in_focused(PluginKind::File, Some(TOUR_PATH.to_string()));
}

const LESSON: &str = r#"=====================================================================
  The Neon Tour
=====================================================================

This is a throwaway buffer. Edit it freely. Nothing here is saved
unless you ask. Work top to bottom. Lines that begin with DO are
exercises: do them right here, on this text.

If you ever feel lost, press Escape to return to normal mode, then
keep reading. The mode shows in the top bar.


---------------------------------------------------------------------
  1. Modes
---------------------------------------------------------------------

Neon starts in normal mode, where keys are commands, not text.
Press  i  to enter insert mode, type, then press  Esc  to go back.

DO: move to the empty line below, press i, type your name, press Esc.



---------------------------------------------------------------------
  2. Moving around
---------------------------------------------------------------------

In normal mode the keys  h j k l  move left, down, up, and right.
The arrow keys work too. For words,  w  goes forward and  b  back.
For line ends,  0  goes to the start and  $  to the end.

DO: travel along the next line and stop on the word you want.
The word you are looking for is over .............. here.


---------------------------------------------------------------------
  3. Deleting
---------------------------------------------------------------------

x  deletes the character under the cursor.
dw  deletes a word.  dd  deletes the whole line.

DO: this line has has a doubled word. Land on the second has, press dw.
DO: delete this entire practice line with dd.


---------------------------------------------------------------------
  4. Inserting
---------------------------------------------------------------------

i  inserts before the cursor and  a  after it.
A  inserts at the end of the line and  o  opens a line below.
I  inserts at the first non-blank character.

DO: press A at the end of the next line and finish the sentence.
The best editor is


---------------------------------------------------------------------
  5. Undo and redo
---------------------------------------------------------------------

u  undoes the last change.  Ctrl-r  redoes it.

DO: delete the next line with dd, then press u to bring it back.
Keep me. I should still be here after you undo.


---------------------------------------------------------------------
  6. The leader
---------------------------------------------------------------------

Press  SPC  in normal mode to open the which-key menu. Every command
lives under it. A few to try:

  SPC f f   open a folder         SPC f s   save
  SPC /     search the project    SPC j w   jump to a word
  SPC ;     toggle a comment      SPC g g   go to definition

DO: press SPC, read the menu, then a letter to dive in or Esc to back out.


---------------------------------------------------------------------
  7. Search
---------------------------------------------------------------------

In normal mode  /  searches inside this buffer.
SPC /  searches across the whole project with ripgrep.

DO: press / then type  leader  and press Enter to find the word above.


---------------------------------------------------------------------
  8. Where to go next
---------------------------------------------------------------------

Press  SPC ?  anytime for the full keybinding reference.
Open the plugin manager to read the keymaps. They are rhai you can
edit live, so every binding here is yours to change.

You are ready. Close this buffer whenever you like.
"#;
