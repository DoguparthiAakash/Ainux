# COB Description Language


alias color colour
alias n north
alias s south
alias e east
alias w west 
alias ne
alias se
alias sw
alias nw
alias c center
alias centre center


import system-colors.cob

Widget inherits System-Color {
   position-x: 100
   position-y: 100
   anchor:     absolute # or n,s,e,w,ne,se,sw,nw,c
   width:      192
   height:     108
   visible:    false
   color:      system-color-frame # Maybe instead System-color.frame?
   # No need to - local 'color' overrides system.color, so just leave it
   # out to get the system color.
}

MouseArea inherits Widget {
   mouse-normal-img:    file-relative:#mouse/normal.png
   mouse-busy-img:      file-relative:#mouse/busy.png
   mouse-link-img:      file-relative:#mouse/link.png
   mouse-nav-img:       file-relative:#mouse/nav.png
   mouse-speed:         105%
   clickable:           yes # Shouldn't every object have a list of callbacks for clicked/pressed/released/scrolled?
   scrollable:          no
}

Text {
   font-face:           Helvetica 
   font-color:          system-color-font
   font-size:           12pt
   font-subscript:      false
   font-superscript:    false
   font-bold:           false
   font-italic:         false
   font-underline:      false
}

Rectangle inherits Widget MouseArea Text {
   text:          "Text Goes Here"  # Quotes accept as a single string
   border-width:  1px
   border-color:  blue
   border-relief: inset # or ridged, beveled, etc
   border-radius: 1px   # for rounded rectangles
}

Button inherits Rectangle {
   scrollable:    no
}

MyButton1 inherits Button {
   text:    "Okay"
   anchor:  west
}

MyButton2 inherits Button {
   text:    "Cancel"
   anchor:  center
}

Frame inherits Rectangle {
   btnOK:         cob:#MyButton1 # Copy; btnOK can be changed/destroyed without affecting MyButton1
   btnCANCEL:     cob:#MyButton2
   btnRETRY:      {              # Linked in - btnRETRY will be destroyed when Frame is destroyed
                     text:    "Retry?"
                     anchor:  east
                  }
   visible:       true # makes it actually appear, C-program can set this too
}
