use project_earth::run;
use pollster;

fn main() {
    pollster::block_on(run());
}
