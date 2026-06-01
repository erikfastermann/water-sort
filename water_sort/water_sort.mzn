enum Color;

int: BottleCount;

int: BottleSize;

int: MaxTime;

array[1..BottleCount, 1..BottleSize] of Color: StartConfig;

array[1..BottleCount] of 0..BottleSize: StartFillHeight;



array[1..MaxTime, 1..BottleCount, 1..BottleSize] of var Color: Config;

array[1..MaxTime, 1..BottleCount] of var 0..BottleSize: FillHeight;

array[1..MaxTime] of var 1..BottleCount: FromBottle;

array[1..MaxTime] of var 1..BottleCount: ToBottle;

var 1..MaxTime: UsedTime;



constraint forall(b in 1..BottleCount, h in 1..BottleSize) (
  Config[1, b, h] == StartConfig[b, h]
);

constraint forall(b in 1..BottleCount) (
  FillHeight[1, b] == StartFillHeight[b]
);



constraint forall(t in 1..UsedTime-1) (
  FromBottle[t] != ToBottle[t]
);

constraint forall(t in 1..UsedTime-1) (
  FillHeight[t, FromBottle[t]] > 0
);

constraint forall(t in 1..UsedTime-1) (
  FillHeight[t, ToBottle[t]] < BottleSize
);

constraint forall(t in 1..UsedTime-1) (
  FillHeight[t, ToBottle[t]] == 0
    \/ Config[t, FromBottle[t], FillHeight[t, FromBottle[t]]]
      == Config[t, ToBottle[t], FillHeight[t, ToBottle[t]]]
);



constraint forall(t in 1..UsedTime-1, b in 1..BottleCount, i in 1..FillHeight[t, b]) (
  b == FromBottle[t] \/ b == ToBottle[t] \/ Config[t, b, i] == Config[t+1, b, i]
);

constraint forall(t in 1..UsedTime-1, b in 1..BottleCount) (
  b == FromBottle[t] \/ b == ToBottle[t] \/ FillHeight[t, b] == FillHeight[t+1, b]
);

constraint forall(t in 1..UsedTime-1) (
  let {
    var 1..BottleSize: ToCapacity = BottleSize - FillHeight[t, ToBottle[t]];
    var Color: TopColor = Config[t, FromBottle[t], FillHeight[t, FromBottle[t]]];
    var 1..BottleSize: TopColorCount = sum(i in 1..FillHeight[t, FromBottle[t]]) (
      bool2int(forall(j in i..FillHeight[t, FromBottle[t]]) (
        Config[t, FromBottle[t], j] == TopColor
      ))
    );
    var 1..BottleSize: SizeDiff = min(ToCapacity, TopColorCount);
    var 0..BottleSize-1: NewFromHeight = FillHeight[t, FromBottle[t]] - SizeDiff;
    var 1..BottleSize: NewToHeight = FillHeight[t, ToBottle[t]] + SizeDiff;
  } in FillHeight[t+1, FromBottle[t]] == NewFromHeight
    /\ FillHeight[t+1, ToBottle[t]] == NewToHeight
    /\ forall(i in 1..NewFromHeight) (
      Config[t, FromBottle[t], i] == Config[t+1, FromBottle[t], i]
    )
    /\ forall(i in 1..FillHeight[t, ToBottle[t]]) (
      Config[t, ToBottle[t], i] == Config[t+1, ToBottle[t], i]
    )
    % The min here silences index out of bounds warnings.
    % Shouldn't the compiler know this never happens?
    /\ forall(i in FillHeight[t, ToBottle[t]]+1..min(NewToHeight, BottleSize)) (
      Config[t+1, ToBottle[t], i] == TopColor
    )
);



constraint forall(b in 1..BottleCount) (
  FillHeight[UsedTime, b] == 0 \/ FillHeight[UsedTime, b] == BottleSize
);

constraint forall(b in 1..BottleCount, i in 1..BottleSize) (
  FillHeight[UsedTime, b] == 0
    \/ Config[UsedTime, b, i] == Config[UsedTime, b, 1]
);



solve minimize UsedTime;
