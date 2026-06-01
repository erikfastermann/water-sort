int: BlockSize;

int: Size = BlockSize ^ 2;

array[1..Size, 1..Size] of var 0..Size: Input;

array[1..Size, 1..Size] of var 1..Size: Sudoku;

constraint forall(r, c in 1..Size where Input[r, c] != 0) (
  Sudoku[r, c] == Input[r, c]
);

constraint forall(r, c1, c2 in 1..Size where c1 < c2) (
  Sudoku[r, c1] != Sudoku[r, c2]
);

constraint forall(r1, r2, c in 1..Size where r1 < r2) (
  Sudoku[r1, c] != Sudoku[r2, c]
);

constraint forall(y, x in 0..BlockSize-1, a, b, c, d in 1..BlockSize where a != c /\ b != d) (
  Sudoku[y*BlockSize + a, x*BlockSize + b] != Sudoku[y*BlockSize + c, x*BlockSize + d]
);

solve satisfy;
