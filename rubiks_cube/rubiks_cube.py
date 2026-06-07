import asyncio
from pathlib import Path

import minizinc


size = 3
max_time = 20
minizinc_processes = 10


W = "W"
R = "R"
G = "G"
B = "B"
Y = "Y"
O = "O"

color = [W, R, G, B, Y, O]

f_front = [
    G, G, O,
    B, G, G,
    G, R, O,
]

f_back = [
    O, O, B,
    Y, B, R,
    B, O, R,
]

f_up = [
    W, W, G,
    W, W, Y,
    R, W, G,
]

f_down = [
    R, W, B,
    Y, Y, O,
    Y, Y, R,
]

f_left = [
    O, B, W,
    B, O, O,
    B, R, Y,
]

f_right = [
    Y, B, W,
    R, R, G,
    Y, G, W,
]

start_position = f_up + f_down + f_right + f_left + f_front + f_back


# HTM (half turn metric)
clockwise = "clockwise"
double = "double"
counterclockwise = "counterclockwise"
kind = [clockwise, double, counterclockwise];

up = "up"
down = "down"
right = "right"
left = "left"
front = "front"
back = "back"

face = [up, down, right, left, front, back]

s = "start"
e = "end"

bound = [(s, s), (s, e), (e, e), (e, s)]
w_bound = bound * 3

def bound_index(b):
    if b == s:
        return 0
    elif b == e:
        return size - 1
    else:
        assert False


corner = [
    [(front, s, s), (up, e, s), (left, s, e)],
    [(front, s, e), (up, e, e), (right, s, s)],
    [(front, e, e), (down, s, e), (right, e, s)],
    [(front, e, s), (down, s, s), (left, e, e)],
    [(back, s, s), (up, s, e), (right, s, e)],
    [(back, s, e), (up, s, s), (left, s, s)],
    [(back, e, e), (down, e, s), (left, e, s)],
    [(back, e, s), (down, e, e), (right, e, e)],
]

def filter_corner(f, b):
    v = [c for c in corner if (f, b[0], b[1]) in c]
    assert len(v) == 1
    return v[0]

def match_corners(f, c1, c2):
    count = { ff: 0 for ff in face }
    for part in c1 + c2:
        count[part[0]] += 1
    assert count[f] == 2
    count[f] = 0
    ff, cc = max(count.items(), key=lambda v: v[1])
    assert cc == 2
    out = [v for v in c1 if v[0] == ff] + [v for v in c2 if v[0] == ff]
    assert len(out) == 2
    assert out[0][1] == out[1][1] or out[0][2] == out[1][2]
    assert out[0][0] == out[1][0]
    return out

def edge_indices(e):
    r0 = bound_index(e[0][1])
    c0 = bound_index(e[0][2])
    r1 = bound_index(e[1][1])
    c1 = bound_index(e[1][2])

    r = range(min(r0, r1), max(r0, r1)+1)
    c = range(min(c0, c1), max(c0, c1)+1)

    r = r if r0 <= r1 else reversed(r)
    c = c if c0 <= c1 else reversed(c)

    out = [(rr, cc) for rr in r for cc in c]
    assert len(out) == 3
    return out

def map_index(move_face, move_kind, input_face, row, column):
    return (
        (face.index(move_face) * 3 * 6 * size * size)
        + (kind.index(move_kind) * 6 * size * size)
        + (face.index(input_face) * size * size)
        + (row * size)
        + column
    )

def mapping_get(mapping_matrix, move_face, move_kind):
    index = (
        (face.index(move_face) * 3 * 6 * size * size)
        + (kind.index(move_kind) * 6 * size * size)
    )

    return mapping_matrix[index:index + (6 * size * size)]

def m_index(input_face, row, column):
    return (
        (face.index(input_face) * size * size)
        + (row * size)
        + column
    )

def rotate_right(row, column):
    return (column, size - 1 - row)

def build_mapping():
    mapping_matrix = [-1] * (6 * 3 * 6 * size * size)

    for f in face:
        for k_index, k in enumerate(kind):
            for r in range(size):
                for c in range(size):
                    rot_r, rot_c = rotate_right(r, c)
                    for _ in range(k_index):
                        rot_r, rot_c = rotate_right(rot_r, rot_c)

                    mapping_index = map_index(f, k, f, r, c)
                    mapping_matrix[mapping_index] = m_index(f, rot_r, rot_c)

            edge_bounds = zip(
                w_bound[:4],
                w_bound[1:5],
                w_bound[1+k_index:5+k_index],
                w_bound[2+k_index:6+k_index],
            )

            found_faces = { f }
            for b1, b2, b3, b4 in edge_bounds:
                e1 = match_corners(f, filter_corner(f, b1), filter_corner(f, b2))
                e2 = match_corners(f, filter_corner(f, b3), filter_corner(f, b4))
                e1_face, e2_face = e1[0][0], e2[0][0]
                e1_indices, e2_indices = edge_indices(e1), edge_indices(e2)

                found_faces.add(e1_face)
                found_faces.add(e2_face)

                for ((r1, c1), (r2, c2)) in zip(e1_indices, e2_indices):
                    mapping_index = map_index(f, k, e1_face, r1, c1)
                    mapping_matrix[mapping_index] = m_index(e2_face, r2, c2)

                for r in range(size):
                    for c in range(size):
                        if (r, c) not in e1_indices:
                            mapping_index = map_index(f, k, e1_face, r, c)
                            mapping_matrix[mapping_index] = m_index(e1_face, r, c)

            assert len(found_faces) == 5

            for f_opposite in face:
                if f_opposite in found_faces:
                    continue

                for r in range(size):
                    for c in range(size):
                        mapping_index = map_index(f, k, f_opposite, r, c)
                        mapping_matrix[mapping_index] = m_index(f_opposite, r, c)

    assert all(v >= 0 for v in mapping_matrix)

    for f in face:
        for k in kind:
            m = mapping_get(mapping_matrix, f, k)
            assert set(m) == set(range(6 * size * size))

    for i in range(len(mapping_matrix)):
        mapping_matrix[i] += 1

    return mapping_matrix

async def run_minizinc():
    for c in color:
        assert sum(1 for cc in start_position if cc == c) == (size ** 2)

    model = minizinc.Model()
    model.add_file(Path("rubiks_cube.mzn"))
    solver = minizinc.Solver.lookup("cp-sat")
    instance = minizinc.Instance(solver, model)

    instance["Size"] = size
    instance["MaxTime"] = max_time
    instance["StartPosition"] = start_position
    instance["MappingIndex"] = build_mapping()

    results = instance.solutions(
        processes=minizinc_processes,
        free_search=True,
        intermediate_solutions=True,
    )
    last_result = None

    async for result in results:
        if result.solution is not None:
            print("--- New solution ---")
            print(f"Status: {result.status}")
            print(f"Used Time: {result['UsedTime']}")
            print(f"Moves: {result['Move'][:result['UsedTime']-1]}")
            print()

    print("--- Search Process Finished ---")
    if last_result:
        print(f"Final Solver Status: {last_result.status}")
    else:
        print(f"No solution found")

if __name__ == "__main__":
    asyncio.run(run_minizinc())
