use crate::maze::Maze;

// normaliza ángulo a [-pi, pi]
#[inline]
fn normalize_angle(mut a: f32) -> f32 {
    while a >  std::f32::consts::PI { a -= 2.0*std::f32::consts::PI; }
    while a < -std::f32::consts::PI { a += 2.0*std::f32::consts::PI; }
    a
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum EnemyState { Patrol, Chase, Cooldown }

pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub a: f32,         // orientación
    pub active: bool,   // aún no aparece hasta que el juego lo active
    pub fov: f32,
    pub range: f32,
    // movimiento
    speed_patrol: f32,
    speed_chase: f32,
    // estado
    state: EnemyState,
    cooldown: f32,          // tiempo restante de cooldown (s)
    cooldown_max: f32,
    // patrulla simple
    patrol_turn_timer: f32, // cada X segundos gira un poco
    // sprite facing con histéresis
    last_face: char,
    // navegación básica (recalcular ruta cada cierto tiempo)
    path_recalc_timer: f32,
    // memoria simple de última posición vista del jugador
    last_seen_x: f32,
    last_seen_y: f32,
    has_last_seen: bool,
    // persistencia de persecución tras perder visión (segundos)
    memory_time: f32,
}

impl Enemy {
    pub fn new(x: f32, y: f32, a: f32) -> Self {
        Self {
            x, y, a,
            active: false,
            // Wider FOV (~120°) so it sees the player more often
            fov: std::f32::consts::PI * (2.0/3.0),
            // Slightly larger range to pick you up sooner
            range: 1100.0,
            speed_patrol: 50.0,
            speed_chase: 115.0,
            state: EnemyState::Patrol,
            cooldown: 0.0,
            cooldown_max: 2.5,
            patrol_turn_timer: 0.0,
            last_face: 'S',
            path_recalc_timer: 0.0,
            last_seen_x: 0.0,
            last_seen_y: 0.0,
            has_last_seen: false,
            memory_time: 0.0,
        }
    }

    /// ¿Está en modo persecución ahora?
    pub fn is_chasing(&self) -> bool {
        matches!(self.state, EnemyState::Chase)
    }

    /// ¿Puede ver al jugador? Usa FOV + línea de visión dentro del rango.
    pub fn sees_player(&self, maze: &Maze, px: f32, py: f32, block_size: usize) -> bool {
        let vx = px - self.x;
        let vy = py - self.y;
        let dist = (vx*vx + vy*vy).sqrt();
        if dist > self.range { return false; }

        // dentro de su FOV
        let target = vy.atan2(vx);
        let ad = normalize_angle(target - self.a).abs();
        if ad > self.fov * 0.5 { return false; }

        // y con línea de visión despejada
        line_of_sight_clear(maze, self.x, self.y, px, py, block_size)
    }

    /// Actualiza lógica simple (patrulla → persigue → cooldown → patrulla)
    pub fn update(&mut self, maze: &Maze, px: f32, py: f32, block_size: usize, dt: f32) {
    if !self.active { return; }
        // transición de estados con memoria de 5s
        let sees_now = self.sees_player(maze, px, py, block_size);
        if sees_now {
            self.last_seen_x = px;
            self.last_seen_y = py;
            self.has_last_seen = true;
            self.state = EnemyState::Chase;
            self.memory_time = 5.0; // mantener persecución 5s aunque pierda visión
            self.cooldown = self.cooldown_max; // refresca cooldown para cuando termine memoria
        } else {
            match self.state {
                EnemyState::Chase => {
                    // perdió visión: mantener persecución mientras haya memoria
                    if self.memory_time > 0.0 {
                        self.memory_time -= dt;
                        // permanecer en Chase para seguir buscando hacia last_seen
                    } else {
                        // se acabó la memoria: entrar a cooldown
                        self.state = EnemyState::Cooldown;
                        self.cooldown = self.cooldown_max;
                        self.has_last_seen = false;
                    }
                }
                EnemyState::Cooldown => {
                    self.cooldown -= dt;
                    if self.cooldown <= 0.0 {
                        self.state = EnemyState::Patrol;
                    }
                }
                EnemyState::Patrol => {}
            }
        }

        match self.state {
            EnemyState::Chase => {
                if sees_now {
                    self.chase(px, py, maze, block_size, dt)
                } else if self.has_last_seen {
                    // dirigirnos hacia la última posición conocida con BFS (búsqueda)
                    self.search_last_seen(maze, block_size, dt);
                }
            }
            EnemyState::Cooldown => self.patrol(maze, block_size, dt, /*slow=*/true),
            EnemyState::Patrol => self.patrol(maze, block_size, dt, /*slow=*/false),
        }
    }

    fn search_last_seen(&mut self, maze: &Maze, block_size: usize, dt: f32) {
        // si estamos muy cerca de esa posición, olvidar
        let dx = self.last_seen_x - self.x;
        let dy = self.last_seen_y - self.y;
        if (dx*dx + dy*dy) < 40.0*40.0 { self.has_last_seen = false; return; }
        // navegación BFS hacia last_seen
        self.path_recalc_timer -= dt;
        if self.path_recalc_timer <= 0.0 {
            self.path_recalc_timer = 0.25;
            if let Some((nx, ny)) = next_step_towards(maze, block_size, self.x, self.y, self.last_seen_x, self.last_seen_y) {
                let target = ny.atan2(nx);
                let mut diff = normalize_angle(target - self.a);
                let max_turn = 2.6 * dt;
                if diff >  max_turn { diff =  max_turn; }
                if diff < -max_turn { diff = -max_turn; }
                self.a = normalize_angle(self.a + diff);
            }
        }
        let speed = self.speed_chase * 0.82;
        let dxm = self.a.cos() * speed * dt;
        let dym = self.a.sin() * speed * dt;
        let _ = try_move_with_slide(maze, block_size, &mut self.x, &mut self.y, dxm, dym);
    }

    fn chase(&mut self, px: f32, py: f32, maze: &Maze, block_size: usize, dt: f32) {
        // girar hacia el jugador
        let target = (py - self.y).atan2(px - self.x);
        let mut diff = normalize_angle(target - self.a);
        let max_turn = 2.8 * dt; // giro por segundo
        if diff >  max_turn { diff =  max_turn; }
        if diff < -max_turn { diff = -max_turn; }
        self.a = normalize_angle(self.a + diff);

    let dxn = px - self.x; let dyn_ = py - self.y;
    let dist2 = dxn*dxn + dyn_*dyn_;
    // ligero empuje extra si está muy cerca para que se note el acercamiento
    let boost = if dist2 < 120.0*120.0 { 1.15 } else { 1.0 };
    let speed = self.speed_chase * boost;
        let dx = self.a.cos() * speed * dt;
        let dy = self.a.sin() * speed * dt;
        try_move_with_slide(maze, block_size, &mut self.x, &mut self.y, dx, dy);
    }

    fn patrol(&mut self, maze: &Maze, block_size: usize, dt: f32, slow: bool) {
        let speed = if slow { self.speed_patrol * 0.6 } else { self.speed_patrol };
        self.patrol_turn_timer -= dt;
        if self.patrol_turn_timer <= 0.0 {
            // pequeño giro pseudo-aleatorio determinista
            self.patrol_turn_timer = 1.2;
            self.a = normalize_angle(self.a + 0.6 - 1.2 * ((self.x as i32 ^ self.y as i32) & 1) as f32);
        }
        // avanza
        let dx = self.a.cos() * speed * dt;
        let dy = self.a.sin() * speed * dt;
        if !try_move_with_slide(maze, block_size, &mut self.x, &mut self.y, dx, dy) {
            // si pega pared, gira menos para evitar "trompo" y espera un poco
            self.a = normalize_angle(self.a + 0.5);
            self.patrol_turn_timer = self.patrol_turn_timer.max(0.2);
        }
    }

    /// Navegación simple por celdas libres mediante BFS hasta la celda del jugador.
    fn navigate_towards(&mut self, maze: &Maze, block_size: usize, px: f32, py: f32, dt: f32) {
        self.path_recalc_timer -= dt;
        // Encontrar siguiente celda hacia el jugador (recalcular cada 0.3s aprox.)
        if self.path_recalc_timer <= 0.0 {
            self.path_recalc_timer = 0.30;
            if let Some((nx, ny)) = next_step_towards(maze, block_size, self.x, self.y, px, py) {
                // orientar hacia el centro de la siguiente celda y moverse
                let target = ny.atan2(nx);
                let mut diff = normalize_angle(target - self.a);
                let max_turn = 2.4 * dt;
                if diff >  max_turn { diff =  max_turn; }
                if diff < -max_turn { diff = -max_turn; }
                self.a = normalize_angle(self.a + diff);
            }
        }
        // avanzar con velocidad de persecución reducida
        let speed = self.speed_chase * 0.85;
        let dx = self.a.cos() * speed * dt;
        let dy = self.a.sin() * speed * dt;
        let _ = try_move_with_slide(maze, block_size, &mut self.x, &mut self.y, dx, dy);
    }

    /// Devuelve la clave de textura (N/E/S/W) según desde dónde lo mira la cámara (jugador).
    pub fn facing_key_for_camera(&mut self, cam_x: f32, cam_y: f32) -> char {
        // ángulo desde enemigo hacia cámara
        let ang_to_cam = (cam_y - self.y).atan2(cam_x - self.x);
        // diferencia vs orientación del ENEMIGO (no del jugador)
    let diff = normalize_angle(ang_to_cam - self.a);
        // mapear a 4 direcciones
        // [-pi,pi] -> N,E,S,W
        // cerca de 0   => "mirando al enemigo de frente" => usamos 'S' (cara del enemigo hacia ti)
        // cerca de pi  => lo ves por la espalda => 'N'
        // cerca de +90 => lo ves por su izquierda => 'E'  (convención)
        // cerca de -90 => lo ves por su derecha  => 'W'
        let deg = diff.to_degrees();

        // Candidato según rangos nominales
        let candidate = if deg > -60.0 && deg <= 60.0 {
            'S'
        } else if deg > 60.0 && deg <= 150.0 {
            'E'
        } else if deg <= -60.0 && deg > -150.0 {
            'W'
        } else {
            'N'
        };

        // Histéresis: mantén la cara anterior mientras el ángulo siga dentro de su
        // rango ampliado por un pequeño margen; así evitamos "corte" en los límites.
        let keep_margin = 12.0; // grados extra para mantener
        let in_keep = |face: char, d: f32| -> bool {
            match face {
                'S' => d > -60.0 - keep_margin && d <= 60.0 + keep_margin,
                'E' => d >  60.0 - keep_margin && d <= 150.0 + keep_margin,
                'W' => d >= -150.0 - keep_margin && d <  -60.0 + keep_margin,
                'N' => d <= -150.0 + keep_margin || d > 150.0 - keep_margin,
                _   => false,
            }
        };

        if in_keep(self.last_face, deg) {
            return self.last_face;
        } else {
            self.last_face = candidate;
            return candidate;
        }
    }
}

/// BFS en grid para obtener el siguiente paso hacia la celda objetivo.
fn next_step_towards(maze: &Maze, block: usize, sx: f32, sy: f32, tx: f32, ty: f32) -> Option<(f32, f32)> {
    let w = maze[0].len();
    let h = maze.len();
    let start = (
        (sx / block as f32).floor() as isize,
        (sy / block as f32).floor() as isize,
    );
    let goal = (
        (tx / block as f32).floor() as isize,
        (ty / block as f32).floor() as isize,
    );
    if start.0 < 0 || start.1 < 0 || goal.0 < 0 || goal.1 < 0 { return None; }
    let (sx_i, sy_i) = (start.0 as usize, start.1 as usize);
    let (gx_i, gy_i) = (goal.0 as usize, goal.1 as usize);
    if sx_i >= w || sy_i >= h || gx_i >= w || gy_i >= h { return None; }

    let passable = |i: usize, j: usize| -> bool {
        if j >= h || i >= w { return false; }
        let c = maze[j][i]; c == ' ' || c == 'g'
    };
    if !passable(sx_i, sy_i) || !passable(gx_i, gy_i) { return None; }

    let mut prev: Vec<Vec<Option<(usize,usize)>>> = vec![vec![None; w]; h];
    let mut q = std::collections::VecDeque::new();
    q.push_back((sx_i, sy_i));
    prev[sy_i][sx_i] = Some((sx_i, sy_i));
    let dirs = [(1,0),(-1,0),(0,1),(0,-1)];
    while let Some((cx, cy)) = q.pop_front() {
        if (cx, cy) == (gx_i, gy_i) { break; }
        for (dx,dy) in dirs {
            let nx = cx as isize + dx; let ny = cy as isize + dy;
            if nx < 0 || ny < 0 { continue; }
            let (nxu, nyu) = (nx as usize, ny as usize);
            if nxu >= w || nyu >= h { continue; }
            if prev[nyu][nxu].is_some() { continue; }
            if !passable(nxu, nyu) { continue; }
            prev[nyu][nxu] = Some((cx, cy));
            q.push_back((nxu, nyu));
        }
    }
    if prev[gy_i][gx_i].is_none() { return None; }
    // retroceder desde goal hasta start para hallar el primer paso
    let mut cur = (gx_i, gy_i);
    let mut last = cur;
    while cur != (sx_i, sy_i) {
        last = cur;
        if let Some(p) = prev[cur.1][cur.0] { cur = p; } else { break; }
    }
    let cx = (last.0 as f32 + 0.5) * block as f32;
    let cy = (last.1 as f32 + 0.5) * block as f32;
    Some((cx - sx, cy - sy))
}

/// Línea de vista: muestreo por el segmento
fn line_of_sight_clear(maze: &Maze, x0: f32, y0: f32, x1: f32, y1: f32, block_size: usize) -> bool {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let step = (block_size as f32 * 0.6).max(5.0);
    let dist = (dx*dx + dy*dy).sqrt();
    let steps = (dist / step).ceil() as i32;

    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        let sx = x0 + dx * t;
        let sy = y0 + dy * t;

        let ci = (sx / block_size as f32).floor() as isize;
        let cj = (sy / block_size as f32).floor() as isize;

        if cj < 0 || ci < 0 { return false; }
        let (ci, cj) = (ci as usize, cj as usize);
        if cj >= maze.len() || ci >= maze[cj].len() { return false; }

        let c = maze[cj][ci];
        if c != ' ' && c != 'g' { return false; } // muro bloquea
    }
    true
}

/// Movimiento con slide simple en grid (como tu jugador)
fn try_move_with_slide(maze: &Maze, block: usize, x: &mut f32, y: &mut f32, dx: f32, dy: f32) -> bool {
    let mut moved = false;
    let nx = *x + dx;
    if is_free_radius(maze, block, nx, *y, 10.0) { *x = nx; moved = true; }
    let ny = *y + dy;
    if is_free_radius(maze, block, *x, ny, 10.0) { *y = ny; moved = true; }
    moved
}

// Check a small radius circle around the enemy, so it can approach closer to walls without clipping
fn is_free_radius(map: &Maze, block: usize, wx: f32, wy: f32, radius: f32) -> bool {
    // sample 8 points around the circle plus center
    let samples = 8;
    if !is_cell_free(map, block, wx, wy) { return false; }
    for k in 0..samples {
        let ang = (k as f32) * (std::f32::consts::TAU / samples as f32);
        let sx = wx + radius * ang.cos();
        let sy = wy + radius * ang.sin();
        if !is_cell_free(map, block, sx, sy) { return false; }
    }
    true
}

#[inline]
fn is_cell_free(map: &Maze, block: usize, wx: f32, wy: f32) -> bool {
    let i = (wx / block as f32).floor() as isize;
    let j = (wy / block as f32).floor() as isize;
    if i < 0 || j < 0 { return false; }
    let (i, j) = (i as usize, j as usize);
    if j >= map.len() || i >= map[0].len() { return false; }
    let c = map[j][i];
    c == ' ' || c == 'g'
}
