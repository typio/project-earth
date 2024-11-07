const SAMPLES: u32 = 1;

const MATERIAL_COUNT = 8;

const VOXEL_COUNT = 32 * 32 * 32;

const gamma = 1 / 2.2;

const voxel_length = 0.1; 

struct Camera {
    inverse_view: mat4x4<f32>,
    inverse_proj: mat4x4<f32>,
}

struct Material {
    albedo: vec3<f32>,
    spacer: f32,
    roughness: f32,
    metallic: f32,
    spacer2: vec2<f32>,
    emission_color: vec3<f32>,
    emission_intensity: f32,
}

// struct VoxelVolume {
//     pos: vec3<f32>,
//     spacer: f32,
//     size: vec3<f32>,
//     spacer2: f32,
//     voxels: array<u32>
// }

struct SceneProps {
    sunIntensity: f32,
    lightsIntensity: f32,
    voxel_count: f32,
    rayOffset: f32,
    rayBounces: f32,
}

@group(0) @binding(0) var pixels: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> screenResolution: vec2<f32>;
@group(0) @binding(2) var<uniform> rayOrigin: vec3<f32>;
@group(0) @binding(3) var<uniform> camera: Camera;
@group(0) @binding(4) var<uniform> sceneProps: SceneProps;
@group(0) @binding(5) var<uniform> random_seed: f32;
// @group(0) @binding(6) var<uniform> materials: array<Material, MATERIAL_COUNT>;
@group(0) @binding(6) var<storage> voxels: array<array<array<u32, 32>, 32>, 32>;

struct Ray {
    o: vec3<f32>,
    d: vec3<f32>
}

struct RayPayload {
    hit: bool,
    objectIndex: u32,

    hitDistance: f32,

    worldPosition: vec3<f32>,
    worldNormal: vec3<f32>
}

fn calculate_ray_direction(coordinate: vec2<u32>) -> vec3<f32> {
    let current_pixel = coordinate;
    let pixel_center = (vec2<f32>(current_pixel) + vec2(.5, .5)) / screenResolution;

    // normalized device coordinate
    let ndc: vec2<f32> = vec2(2., -2.) * pixel_center + vec2(-1., 1.);
    let ray_target: vec4<f32> = camera.inverse_proj * vec4<f32>(ndc.x, ndc.y, 1., 1.);
    let pixel_ray_direction: vec4<f32> = camera.inverse_view * vec4<f32>(
        normalize(vec3<f32>(ray_target.xyz) / ray_target.w),
        0.
    );

    return pixel_ray_direction.xyz;
}

struct Volume {
    pos: vec3<f32>,
    size: vec3<f32>,
    voxel_length: f32
}

struct Intersection {
    intersects: bool,
    tmin: f32,
    tmax: f32,
}

const SUN_DIRECTION: vec3<f32> = vec3<f32>(0.8, 1.0, 0.6);
const SUN_COLOR: vec3<f32> = vec3<f32>(1.0, 0.95, 0.8);
const AMBIENT_LIGHT: vec3<f32> = vec3<f32>(0.1, 0.15, 0.2);

fn calculate_lighting(normal: vec3<f32>, albedo: vec3<f32>) -> vec3<f32> {
    let NdotL = max(dot(normal, normalize(SUN_DIRECTION)), 0.0);

    let up_dot = max(dot(normal, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    let ao = mix(0.5, 1.0, up_dot);

    return (SUN_COLOR * NdotL * sceneProps.sunIntensity + AMBIENT_LIGHT * ao) * albedo;
}

fn intersect_volume(ray: Ray, volume: Volume) -> Intersection {
    var intersection: Intersection;

    var tmin: f32 = -10000000000;
    var tmax: f32 = 10000000000;

    intersection.intersects = false;
    intersection.tmin = tmin;
    intersection.tmax = tmax;

    let volume_min = vec3<f32>(volume.pos);
    let volume_max = vec3<f32>(volume.pos + (volume.size * volume.voxel_length));

    for (var i = 0; i < 3; i++) {
        if abs(ray.d[i]) < 0.00000001 {
            if ray.o[i] < volume_min[i] || ray.o[i] > volume_max[i] {
                return intersection;
            }
        } else {
            let inv_dir = 1.0 / ray.d[i];
            var t1 = (volume_min[i] - ray.o[i]) * inv_dir;
            var t2 = (volume_max[i] - ray.o[i]) * inv_dir;

            if t1 > t2 {
                let temp_t1 = t1;
                t1 = t2;
                t2 = temp_t1;
            }

            tmin = max(t1, tmin);
            tmax = min(t2, tmax);

            if tmin > tmax {
                return intersection;
            }
        }
    }

    if tmax < 0.0 {
        return intersection;
    }

    intersection.tmin = max(0.0, tmin);
    intersection.tmax = tmax;
    intersection.intersects = true;
    return intersection;
}

fn trace_ray(ray: Ray) -> RayPayload {
    var rayPayload: RayPayload;
    var volume: Volume;
    volume.pos = vec3<f32>(-1.6, -1.6, -1.6);
    volume.size = vec3<f32>(32.0, 32.0, 32.0);
    volume.voxel_length = 0.1;

    let intersect_result: Intersection = intersect_volume(ray, volume);

    if intersect_result.intersects {
        var found_voxel: u32 = 0u;
        var normal: vec3<f32>;

        let world_to_voxel = (ray.o - volume.pos) / volume.voxel_length;
        var voxel_pos = world_to_voxel + intersect_result.tmin * ray.d / volume.voxel_length;

        var x = i32(floor(voxel_pos.x));
        var y = i32(floor(voxel_pos.y));
        var z = i32(floor(voxel_pos.z));

        let x_step: i32 = select(-1, 1, ray.d.x > 0.0);
        let y_step: i32 = select(-1, 1, ray.d.y > 0.0);
        let z_step: i32 = select(-1, 1, ray.d.z > 0.0);

        var dt_x = abs(volume.voxel_length / ray.d.x);
        var dt_y = abs(volume.voxel_length / ray.d.y);
        var dt_z = abs(volume.voxel_length / ray.d.z);

        var t_max_x = select(
            (floor(voxel_pos.x) + 1.0 - voxel_pos.x),
            (voxel_pos.x - floor(voxel_pos.x)),
            ray.d.x < 0.0
        ) * abs(volume.voxel_length / ray.d.x);

        var t_max_y = select(
            (floor(voxel_pos.y) + 1.0 - voxel_pos.y),
            (voxel_pos.y - floor(voxel_pos.y)),
            ray.d.y < 0.0
        ) * abs(volume.voxel_length / ray.d.y);

        var t_max_z = select(
            (floor(voxel_pos.z) + 1.0 - voxel_pos.z),
            (voxel_pos.z - floor(voxel_pos.z)),
            ray.d.z < 0.0
        ) * abs(volume.voxel_length / ray.d.z);

        let x_out = select(-1, 32, x_step > 0);
        let y_out = select(-1, 32, y_step > 0);
        let z_out = select(-1, 32, z_step > 0);

        var iters = 0u;
        while found_voxel == 0u && iters < 1000u {
            if x >= 0 && x < 32 && y >= 0 && y < 32 && z >= 0 && z < 32 {
                found_voxel = voxels[x][y][z];
            }

            if found_voxel != 0u {
                break;
            }

            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    x += x_step;
                    if x == x_out { break; }
                    t_max_x += dt_x;
                    normal = vec3<f32>(-f32(x_step), 0.0, 0.0);
                } else {
                    z += z_step;
                    if z == z_out { break; }
                    t_max_z += dt_z;
                    normal = vec3<f32>(0.0, 0.0, -f32(z_step));
                }
            } else {
                if t_max_y < t_max_z {
                    y += y_step;
                    if y == y_out { break; }
                    t_max_y += dt_y;
                    normal = vec3<f32>(0.0, -f32(y_step), 0.0);
                } else {
                    z += z_step;
                    if z == z_out { break; }
                    t_max_z += dt_z;
                    normal = vec3<f32>(0.0, 0.0, -f32(z_step));
                }
            }

            iters++;
        }

        if found_voxel != 0u {
            var rayPayload: RayPayload;
            rayPayload.hit = true;
            rayPayload.objectIndex = found_voxel;
            let voxel_world_pos = volume.pos + vec3<f32>(f32(x), f32(y), f32(z)) * volume.voxel_length;
            rayPayload.worldPosition = voxel_world_pos;
            rayPayload.worldNormal = normalize(normal);
            return rayPayload;
        }
    }

    return miss(ray);
}

fn chit(ray: Ray, hitDistance: f32, objectIndex: u32) -> RayPayload {
    var rayPayload: RayPayload;
    rayPayload.hitDistance = hitDistance;
    rayPayload.hit = true;
    rayPayload.objectIndex = objectIndex;

    // let closestSphere = spheres[objectIndex];
    // let origin: vec3<f32> = ray.o - closestSphere.pos;
    // rayPayload.worldPosition = origin + ray.d * hitDistance;
    // rayPayload.worldNormal = normalize(rayPayload.worldPosition);
    // rayPayload.worldPosition += closestSphere.pos;

    return rayPayload;
}

fn miss(ray: Ray) -> RayPayload {
    var rayPayload: RayPayload;
    rayPayload.hit = false;
    rayPayload.hitDistance = -1.;
    return rayPayload;
}

// classic noise hash function
fn rand(x: u32, seed: f32) -> f32 {
    return fract(sin(dot(
        vec2(f32(x) / exp2(14.0), seed),
        vec2(12.9898, 78.233)
    )) * 43758.5453);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<u32>(global_id.x % u32(screenResolution.x), global_id.x / u32(screenResolution.x));
    if global_id.x > u32(screenResolution.x * screenResolution.y) {
        textureStore(pixels, coords, vec4<f32>(0.0, 0.0, 1.0, 1.0));
        return;
    }

    let rayDirection = calculate_ray_direction(coords);

    var light = vec3<f32>(0., 0., 0.);

    for (var sample = 0u; sample < SAMPLES; sample++) {
        var contribution = vec3<f32>(1., 1., 1.);
        var ray: Ray;
        ray.d = rayDirection;
        ray.o = rayOrigin;
        for (var i = 0u; i < u32(sceneProps.rayBounces); i++) {
            ray.o += (rand(global_id.x * i, random_seed) - 0.5) * sceneProps.rayOffset;

            let rayPayload: RayPayload = trace_ray(ray);
            if !rayPayload.hit { 
                // light += vec3(0.53, 0.8, 0.92) * contribution; // hit background
                break;
            }

            let reflectedDirection = reflect(ray.d, rayPayload.worldNormal);
            let randomVector = normalize(vec3(rand(global_id.x + i, random_seed) - 0.5,
                rand(global_id.x * 2 + i, random_seed) - 0.5,
                rand(global_id.x * 3 + i, random_seed) - 0.5));

            if rayPayload.hit {
                var voxel_color: vec3<f32>;

                switch rayPayload.objectIndex {

                case 1u { // WATER
                        voxel_color = vec3<f32>(0.02, 0.03, 0.1);
            }
                case 2u { // GRASS
                        voxel_color = vec3<f32>(0.1, 0.3, 0.05);
            }
                case 3u { // DIRT
                        voxel_color = vec3<f32>(0.25, 0.15, 0.08);
            }
                case 4u { // STONE
                        voxel_color = vec3<f32>(0.3, 0.3, 0.32);
            }
                case default {
                        voxel_color = vec3<f32>(1.0, 0.0, 0.0);
                    }
            }

                var shadow_ray: Ray;
                shadow_ray.o = rayPayload.worldPosition + rayPayload.worldNormal * 0.0001;
                shadow_ray.d = normalize(SUN_DIRECTION);
                let shadow_result = trace_ray(shadow_ray);

                let direct_light = calculate_lighting(rayPayload.worldNormal, voxel_color);

                let in_shadow = shadow_result.hit;
                let final_light = select(direct_light, direct_light * 0.1, in_shadow);

                light += contribution * final_light;

                contribution *= voxel_color * 1.0;

                ray.o = rayPayload.worldPosition + rayPayload.worldNormal * 0.0001;

                let roughness = 0.0;
                ray.d = normalize(mix(reflectedDirection, randomVector, roughness));
            // ray.d = normalize(ray.d + rayPayload.worldNormal * 0.1); // Slight bias towards normal

            // light += contribution * voxel_color;
            //
            // contribution *= 0.5; 
            //
            // // move ray to hit for next, but a lil away so it doesnt collide with the inside
            // ray.o = rayPayload.worldPosition + rayPayload.worldNormal * 0.0001;
            //
            // let roughness = 0.0;
            // ray.d = normalize(mix(reflectedDirection, randomVector, roughness));
            // ray.d = normalize(ray.d + rayPayload.worldNormal);
            }
        }
    }


    var finalColor = light / f32(SAMPLES);
    finalColor = pow(finalColor, vec3(gamma, gamma, gamma)); // gamma correction

    textureStore(pixels, coords, vec4(finalColor, 1.0));

    return;
}
