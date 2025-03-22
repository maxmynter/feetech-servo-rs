//! # Teleoperation Example
//!
//! This example demonstrates a simple "leader-follower" system with two sets of Feetech servos, for example in the S0-100 robots.
//! The leader servos send their position data to the follower servos, which then mimic the leader's angles.
//!
//! ## What this example does:
//!
//! - Connects to two serial ports (`/dev/ttyACM0` and `/dev/ttyACM1`)
//! - Reads the current positions of 6 leader servos
//! - Converts those positions into angles relative to a calibrated "zero"
//! - Converts the angles back into step positions for the follower servos
//! - Commands the follower servos to move to those target positions
//! - The output will show the current positions of each of the motors, along with the target angles.
//! - On exiting with Ctrl-C will set the torque on all of the motors to zero
//!
//! ## How to run
//!
//! ```bash
//! cargo run --example leader_follower
//! ```
//!
//! ## Notes
//!
//! - Make sure `/dev/ttyACM0` and `/dev/ttyACM1` match your actual connected serial ports.
//! - Adjust `leader_zero` and `follower_zero` arrays to your servo calibration.
//! - Press **Ctrl+C** to safely stop the loop.

use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{io, thread::sleep, time::Duration};

use feetech_servo_rs::{Command, Driver};

fn main() {
    let mut leader = Driver::new("/dev/ttyACM0");
    let mut follower = Driver::new("/dev/ttyACM1");

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    ctrlc::set_handler(move || {
        println!("\n\n\n\nCtrl+C detected, shutting down...");
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let leader_zero = vec![2123, 4080, 4016, 2168, 1681, 1365];
    let follower_zero = vec![2025, 3082, 934, 2111, 1934, 1865];

    let mut leader_positions: Vec<u16> = [0; 6].to_vec();
    let mut follower_positions: Vec<u16> = [0; 6].to_vec();

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        for motor_id in 1u8..=6u8 {
            leader_positions[(motor_id - 1) as usize] =
                leader.act(motor_id, Command::ReadCurrentPosition).unwrap();

            follower_positions[(motor_id - 1) as usize] = follower
                .act(motor_id, Command::ReadCurrentPosition)
                .unwrap();
        }
        let leader_angles: Vec<i32> = leader_positions
            .iter()
            .zip(&leader_zero)
            .map(|(step, zero)| step_to_angle(*step as i32, *zero))
            .collect();

        let target_positions: Vec<u16> = leader_angles
            .iter()
            .zip(&follower_zero)
            .map(|(target, zero)| angle_to_step(*target, *zero))
            .collect();

        for motor_id in 1u8..=6u8 {
            follower
                .act(
                    motor_id,
                    Command::WriteTargetPosition(target_positions[(motor_id - 1) as usize]),
                )
                .unwrap();
        }
        println!(
            "\rLeader positions: {:?}\nFollower positions: {:?}\n angle: {:?}         ",
            leader_positions, follower_positions, leader_angles,
        );
        print!("\x1b[3A");
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(1));
    }

    // Clean up by turning off all the motors
    for motor_id in 1u8..=6u8 {
        leader
            .act(motor_id, Command::WriteTorqueSwitch(false))
            .unwrap();
        follower
            .act(motor_id, Command::WriteTorqueSwitch(false))
            .unwrap();
    }
}

const STEPS_PER_REV: i32 = 4096;
const DEGREES_PER_STEP: f32 = 360.0 / STEPS_PER_REV as f32;

fn step_to_angle(current_step: i32, zero_step: i32) -> i32 {
    let mut delta = (current_step - zero_step) % STEPS_PER_REV;
    if delta < 0 {
        delta += STEPS_PER_REV;
    }
    let mut angle = (delta as f32 * DEGREES_PER_STEP).round() as i32;
    if angle > 180 {
        angle -= 360;
    }
    angle
}

fn angle_to_step(angle: i32, zero_step: i32) -> u16 {
    if !(-180..=180).contains(&angle) {
        panic!("Angle must be between -180 and 180 degrees");
    }
    let mut angle_norm = angle;
    if angle_norm < 0 {
        angle_norm += 360;
    }
    let step_offset = ((angle_norm as f32) / DEGREES_PER_STEP).round() as i32;
    (zero_step + step_offset).rem_euclid(STEPS_PER_REV) as u16
}
