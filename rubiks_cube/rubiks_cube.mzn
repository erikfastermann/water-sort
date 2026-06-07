include "globals.mzn";


int: Size;

int: MaxTime;

array[1..(6 * Size * Size)] of Color: StartPosition;

array[1..(6 * 3 * 6 * Size * Size)] of 1..(6 * Size * Size): MappingIndex;


enum Color = { W, R, G, B, Y, O };

array[1..MaxTime, 1..(6 * Size * Size)] of var Color: History;

array[1..MaxTime] of var 1..(6 * 3): Move;

var 1..MaxTime: UsedTime;



constraint forall(i in 1..(6 * Size * Size)) (
  History[1, i] == StartPosition[i]
);

constraint forall(t in 1..(UsedTime-1), i in 1..(6 * Size * Size)) (
  History[t, i] == History[t+1, MappingIndex[(Move[t] - 1) * (6 * Size * Size) + i]]
);

constraint forall(f in 1..6) (
  all_equal(i in 1..(Size * Size)) (History[UsedTime, (f - 1) * (Size * Size) + i])
);



solve minimize UsedTime;
