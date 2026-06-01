int: Height;

int: Width;

array[1..Height, 1..Width] of Field: Grid;

1..Height: StartRow;

1..Width: StartColumn;

Direction: StartDirection;

1..Height: EndRow;

1..Width: EndColumn;

int: MaxStates;

int: MaxSteps;



enum Field = { T, G }; % T = Tree, G = Ground

enum Check = { Leaf, TreeFront, TreeLeft, TreeRight };

enum Action = { Stay, GoForward, TurnLeft, TurnRight };

enum Direction = { North, East, South, West };

array[1..MaxStates] of var Check: Condition;

array[1..MaxStates] of var Action: YesAction;

array[1..MaxStates] of var Action: NoAction;

array[1..MaxStates] of var 0..MaxStates: YesNext;

array[1..MaxStates] of var 0..MaxStates: NoNext;

var 1..MaxStates: UsedStates;



array[1..MaxSteps] of var 0..MaxStates: State;

array[1..MaxSteps] of var 1..Height: Row;

array[1..MaxSteps] of var 1..Width: Column;

array[1..MaxSteps] of var Direction: CurrentDirection;

var 1..MaxSteps: UsedSteps;



constraint Row[1] == StartRow;

constraint Column[1] == StartColumn;

constraint CurrentDirection[1] == StartDirection;

constraint State[1] == 1;

constraint Row[UsedSteps] == EndRow;

constraint Column[UsedSteps] == EndColumn;

constraint State[UsedSteps] == 0;

constraint forall(s in 1..UsedSteps) (
  Grid[Row[s], Column[s]] == G
);

constraint forall(s in 1..UsedSteps-1) (
  State[s] != 0
);

constraint forall(s in 1..UsedSteps) (
  State[s] <= UsedStates
);



array[Direction] of Direction: DirectionLeft = [
  North: West,
  East: North,
  South: East,
  West: South,
];

array[Direction] of Direction: DirectionRight = [
  North: East,
  East: South,
  South: West,
  West: North,
];

function var 1..Height: front_row(var 1..Height: r, var Direction: d) =
  if d == North then
    if r == 1 then Height else r - 1 endif
  elseif d == South then
    if r == Height then 1 else r + 1 endif
  else
    r
  endif;

function var 1..Width: front_column(var 1..Width: c, var Direction: d) =
  if d == East then
    if c == Width then 1 else c + 1 endif
  elseif d == West then
    if c == 1 then Width else c - 1 endif
  else
    c
  endif;

constraint forall(s in 1..UsedSteps-1) (
  let {
    var Check: Do = Condition[State[s]];

    var Direction: NewLeft = DirectionLeft[CurrentDirection[s]];

    var Direction: NewRight = DirectionRight[CurrentDirection[s]];

    var 1..Height: FrontRow = front_row(Row[s], CurrentDirection[s]);

    var 1..Height: LeftRow = front_row(Row[s], NewLeft);

    var 1..Height: RightRow = front_row(Row[s], NewRight);

    var 1..Width: FrontColumn = front_column(Column[s], CurrentDirection[s]);

    var 1..Width: LeftColumn = front_column(Column[s], NewLeft);

    var 1..Width: RightColumn = front_column(Column[s], NewRight);

    var bool: IsYes = if Do == Leaf then
      Row[s] == EndRow /\ Column[s] == EndColumn
    elseif Do == TreeFront then
      Grid[FrontRow, FrontColumn] == T
    elseif Do == TreeLeft then
      Grid[LeftRow, LeftColumn] == T
    elseif Do == TreeRight then
      Grid[RightRow, RightColumn] == T
    else
      false % TODO
    endif;

    var Action: NextAction = if IsYes then
      YesAction[State[s]]
    else
      NoAction[State[s]]
    endif;

    var 0..MaxStates: NextState = if IsYes then
      YesNext[State[s]]
    else
      NoNext[State[s]]
    endif;

  } in State[s+1] == NextState
    /\ if NextAction == Stay then
      CurrentDirection[s+1] == CurrentDirection[s]
        /\ Row[s+1] == Row[s]
        /\ Column[s+1] == Column[s]
    elseif NextAction == GoForward then
      CurrentDirection[s+1] == CurrentDirection[s]
        /\ Row[s+1] == FrontRow
        /\ Column[s+1] == FrontColumn
    elseif NextAction == TurnLeft then
      CurrentDirection[s+1] == NewLeft
        /\ Row[s+1] == Row[s]
        /\ Column[s+1] == Column[s]
    elseif NextAction == TurnRight then
      CurrentDirection[s+1] == NewRight
        /\ Row[s+1] == Row[s]
        /\ Column[s+1] == Column[s]
    else
      false % TODO
    endif
);



solve minimize UsedStates;
