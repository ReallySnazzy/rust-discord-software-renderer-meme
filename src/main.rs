use minifb::{Key, Window, WindowOptions};
use std::vec::Vec;
use std::fs::read_to_string;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Debug)]
struct CoolCanvasColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl CoolCanvasColor {
    fn mult_scalar(&self, scale: f32) -> CoolCanvasColor {
        CoolCanvasColor {
            red: (self.red as f32 * scale) as u8,
            green: (self.green as f32 * scale) as u8,
            blue: (self.blue as f32 * scale) as u8
        }
    }
}

struct CoolCanvas {
    width: i32, 
    height: i32,
    pixels: Vec<CoolCanvasColor>,
    depth_buffer: Vec<f32>,
    offset_x: i32,
    offset_y: i32,
    resolution_x: i32,
    resolution_y: i32,
}

#[derive(Debug, Clone)]
struct Vector2 {
    x: f32,
    y: f32
}

impl Vector2 {
    fn scale(&self, factor: f32) -> Vector2 {
        Vector2 {
            x: self.x * factor,
            y: self.y * factor
        }
    }

    fn sub(&self, other: &Vector2) -> Vector2 {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y
        }
    }

    fn cross(&self, other: &Vector2) -> Vector3 {
        Vector3 {
            x: 0f32,
            y: 0f32,
            z: self.x * other.y
        }
    }
}

#[derive(Debug, Clone)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32
}

impl Vector3 {
    fn flat(&self) -> Vector3 {
        // fuck fov
        return Vector3 {
            x: self.x / self.z,
            y: self.y / self.z,
            z: 1f32,
        }
    }

    fn dot(&self, other: &Vector3) -> f32 {
        return self.x*other.x + self.y*other.y + self.z*other.z;
    }

    fn unit(&self) -> Vector3 {
        let mag = (self.x*self.x + self.y*self.y + self.z*self.z).sqrt();
        Vector3 {
            x: self.x/mag,
            y: self.y/mag,
            z: self.z/mag
        }
    }

    fn negate(&self) -> Vector3 {
        Vector3 {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }

    fn rotate_y(&self, delta: f32) -> Vector3 {
        Vector3 {
            x: self.x * delta.cos() + self.z * delta.sin(),
            y: self.y,
            z: -self.x * delta.sin() + self.z * delta.cos()
        }
    }

    fn scale(&self, magnitude: f32) -> Vector3 {
        Vector3 {
            x: self.x * magnitude,
            y: self.y * magnitude,
            z: self.z * magnitude
        }
    }

    fn cross(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.y*other.z - self.z*other.y,
            y: self.z*other.x - self.x*other.z,
            z: self.x*other.y - self.y*other.x
        }
    }

    fn sub(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z
        }
    }
}

#[derive(Debug, Clone)]
struct Triangle<T> {
    a: T,
    b: T,
    c: T,
    color: CoolCanvasColor,
    normal: T
}

fn area_tri(a: &Vector3, b: &Vector3, c: &Vector3) -> f32 {
    return (
        (a.x*(b.y-c.y) 
        + b.x*(c.y-a.y) 
        + c.x*(a.y-b.y))/2.0f32
    ).abs();
}

fn flatten_tri(tri: &Triangle<Vector3>) -> Triangle<Vector3> {
    // fuck fov
    return Triangle {
        a: tri.a.flat(),
        b: tri.b.flat(),
        c: tri.c.flat(),
        color: tri.color.clone(),
        normal: tri.normal.flat()
    }
}

fn tri_contains(a: &Vector3, b: &Vector3, c: &Vector3, point: &Vector3) -> bool {
    let area = area_tri(a, b, c);
    let area_a = area_tri(point, b, c);
    let area_b = area_tri(a, point, c);
    let area_c = area_tri(a, b, point);
    return ((area_a + area_b + area_c) - area).abs() < 0.01;
}

fn tri_coord(a: &Vector3, b: &Vector3, c: &Vector3, point: &Vector3) -> f32 {
    let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    let l1 = ((b.y - c.y) * (point.x - c.x) + (c.x - b.x) * (point.y - c.y)) / det;
    let l2 = ((c.y - a.y) * (point.x - c.x) + (a.x - c.x) * (point.y - c.y)) / det;
    let l3 = 1.0f32 - l1 - l2;
    return l1 * a.z + l2 * b.z + l3 * c.z;
}

impl CoolCanvas {
    fn new(width: i32, height: i32, offset_x: i32, offset_y: i32, resolution_x: i32, resolution_y: i32) -> CoolCanvas {
        let mut pixels = vec![];
        let mut depth_buffer = vec![];
        for _ in 0..(width * height) {
            pixels.push(CoolCanvasColor {
                red: 0,
                green: 0,
                blue: 0
            });
            depth_buffer.push(0f32);
        }
        return CoolCanvas {
            width,
            height,
            pixels,
            offset_x,
            offset_y,
            resolution_x,
            resolution_y,
            depth_buffer
        }
    }

    fn clear(&mut self) {
        for pixel in &mut self.pixels {
            pixel.red = 0;
            pixel.green = 0;
            pixel.blue = 0;
        }
        for i in 0..self.depth_buffer.len() {
            self.depth_buffer[i] = 0f32;
        }
    }
    
    fn draw_triangle_3d(&mut self, tri: &Triangle<Vector3>) {
        let light = Vector3{ x: 0.3f32, y: 0.3f32, z: 2f32 }.unit();
        let front = Vector3{ x: 0f32, y: 0f32, z: -1f32 }.unit();

        let mut triangle: Triangle<Vector3> = tri.clone();

        triangle.a.z += 1f32;
        triangle.b.z += 1f32;
        triangle.c.z += 1f32;

        let scale = (self.resolution_x/2) as f32;
        let triangle_2d = flatten_tri(&triangle);
        let a = triangle_2d.a.scale(scale);
        let b = triangle_2d.b.scale(scale);
        let c = triangle_2d.c.scale(scale);
        let normal = a.sub(&c).cross(&b.sub(&c)).unit().negate();
        let min_x = a.x.min(b.x.min(c.x));
        let max_x = a.x.max(b.x.max(c.x));
        let min_y = a.y.min(b.y.min(c.y));
        let max_y = a.y.max(b.y.max(c.y));
        let light_factor = triangle.normal.dot(&light) * 0.4f32 + 0.6f32;
        let mut culling = false;

        if normal.dot(&front) > 0f32 {
            culling = true;
        }

        for x in 0..self.width {
            let global_x = x + self.offset_x - self.resolution_x / 2;
            if (global_x as f32) < min_x || (global_x as f32) > max_x {
                continue;
            }
            for y in 0..self.height {
                let global_y = y + self.offset_y - self.resolution_y / 2;
                if (global_y as f32) < min_y || (global_y as f32) > max_y {
                    continue;
                }
                let draw_coord = Vector3 { x: global_x as f32, y: global_y as f32, z: 0f32 };
                let z = 1000f32 * scale - tri_coord(
                    &triangle.a.scale(scale), 
                    &triangle.b.scale(scale), 
                    &triangle.c.scale(scale), 
                    &draw_coord
                );
                let canvas_index = y as usize * self.width as usize + x as usize;
                if tri_contains(&a, &b, &c, &draw_coord) && z >= self.depth_buffer[canvas_index] {
                    self.depth_buffer[canvas_index] = z;
                    self.pixels[canvas_index] = if !culling {
                        triangle_2d.color.mult_scalar(light_factor)
                    } else {
                        CoolCanvasColor { red: 255, green: 0, blue: 0 }
                    }
                    // let normal_unit = triangle.normal.unit();
                    // self.pixels[canvas_index] = CoolCanvasColor {
                    //     red: (normal_unit.x*255 as f32) as u8,
                    //     green: (normal_unit.y*255 as f32) as u8,
                    //     blue: (normal_unit.z*255 as f32) as u8,
                    // }.mult_scalar(normal_unit.dot(&front));
                }
            }
        }
    }
}

const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;

fn load_obj(path: &str) -> Vec<Triangle<Vector3>> {
    let mut result = Vec::new();
    let mut verticies = Vec::new();
    let mut normals = Vec::new();
    for line in read_to_string(path).unwrap().lines() {
        let args: Vec<&str> = line.split(" ").collect();
        if args.len() < 1 {
            continue;
        }
        if args[0] == "v" {
            verticies.push(
                Vector3 {
                    x: args[1].parse::<f32>().unwrap(),
                    y: -args[2].parse::<f32>().unwrap(),
                    z: -args[3].parse::<f32>().unwrap()
                }
            )
        } else if args[0] == "vn" {
            normals.push(
                Vector3 {
                    x: args[1].parse::<f32>().unwrap(),
                    y: args[2].parse::<f32>().unwrap(),
                    z: args[3].parse::<f32>().unwrap()
                }
            )
        } else if args[0] == "f" {
            let a: Vec<&str> = args[1].split('/').collect(); 
            let b: Vec<&str> = args[2].split('/').collect(); 
            let c: Vec<&str> = args[3].split('/').collect(); 
            let vert_a = verticies[(a[0].parse::<i32>().unwrap() - 1) as usize].clone();
            let vert_b = verticies[(b[0].parse::<i32>().unwrap() - 1) as usize].clone();
            let vert_c = verticies[(c[0].parse::<i32>().unwrap() - 1) as usize].clone();
            let normal = normals[(a[2].parse::<i32>().unwrap() - 1) as usize].clone();
            // let normal = vert_b.sub(&vert_a).cross(&vert_c.sub(&vert_a)).unit().negate();
            let tri = Triangle {
                a: vert_a,
                b: vert_b,
                c: vert_c,
                color: CoolCanvasColor { 
                    red: 255u8, 
                    green: 255u8, 
                    blue: 255u8
                },
                normal
            };
            result.push(tri);
        }
    }
    return result;
}

const THREADS: i32 = 16;

fn main() {
    let mut buffer: Vec<u32> = vec![0; (WIDTH * HEIGHT) as usize];
    let mut window = Window::new("Test", WIDTH as usize, HEIGHT as usize, WindowOptions::default()).unwrap();
    window.limit_update_rate(
        Some(std::time::Duration::from_micros(16000))
    );
    let monkey: Arc<RwLock<Vec<Triangle<Vector3>>>> = Arc::from(RwLock::new(load_obj("./triangles.obj")));
    let mut canvases = vec![];
    for x in 0..(THREADS/2) {
        for y in 0..=1 {
            let sliver_width = WIDTH/(THREADS/2);
            let sliver_height = HEIGHT/2;
            canvases.push(
                Arc::from(Mutex::new(
                    CoolCanvas::new(sliver_width, sliver_height, x*sliver_width, y*sliver_height, WIDTH, HEIGHT)
                ))
            );
        }
    }
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut threads = vec![];
        let d_down = window.is_key_down(Key::D);
        let a_down = window.is_key_down(Key::A);
        if a_down || d_down {
            for tri in &mut *monkey.write().unwrap() {
                let delta = if a_down && !d_down {
                    0.05f32
                } else if !a_down && d_down {
                    -0.05f32
                } else {
                    0f32
                };
                tri.a = tri.a.rotate_y(delta);
                tri.b = tri.b.rotate_y(delta);
                tri.c = tri.c.rotate_y(delta);
                tri.normal = tri.normal.rotate_y(-delta);
            }
            monkey.write().unwrap().sort_by(|tri1, tri2| tri2.a.z.partial_cmp(&tri1.a.z).unwrap());
        }
        let q_down = window.is_key_down(Key::Q);
        let e_down = window.is_key_down(Key::E);
        if q_down || e_down {
            for tri in &mut *monkey.write().unwrap() {
                let delta = if e_down && !q_down {
                    0.05f32
                } else if !e_down && q_down {
                    -0.05f32
                } else {
                    0f32
                };
                tri.a.x += delta;
                tri.b.x += delta;
                tri.c.x += delta;
            }
            monkey.write().unwrap().sort_by(|tri1, tri2| tri2.a.z.partial_cmp(&tri1.a.z).unwrap());
        }
        for canvas in &canvases {
            let canvas = canvas.clone();
            let monkey = monkey.clone();
            threads.push(std::thread::spawn(move || {
                let mut canvas = canvas.lock().unwrap();
                canvas.clear();
                for tri in &*monkey.read().unwrap() {
                    canvas.draw_triangle_3d(tri);
                }
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
        for canvas in &canvases {
            let canvas = canvas.clone();
            let canvas_guard = canvas.lock().unwrap();
            for x in canvas_guard.offset_x..(canvas_guard.offset_x+canvas_guard.width) {
                for y in canvas_guard.offset_y..(canvas_guard.offset_y+canvas_guard.height) {
                    let color = &canvas_guard.pixels[((y - canvas_guard.offset_y)*canvas_guard.width + (x - canvas_guard.offset_x)) as usize];
                    buffer[(WIDTH * y + x) as usize] = color.blue as u32 + color.green as u32*256 + color.red as u32*256*256;
                }
            }
        }
        window
            .update_with_buffer(&buffer, WIDTH as usize, HEIGHT as usize)
            .unwrap();
    }
}
