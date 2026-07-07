# Simple PCB for organising controller work

Just a PCB to organise the controller, isn't necessary for functionality but definitely needed for sanity. Pi Pico symbol and footprint taken from Ki Lime Pi (https://github.com/recursivenomad/ki-lime-pi-to-go), otherwise the rest is default KiCAD 10.0.

(When I say 12V, I really mean the direct battery output, which ranges from ~13.0V to ~16.0V)

## Inputs 

The important part is the input, which are the top 3pin terminal block with 12V, GND and 5V, and the 4pin terminal block with MotorA and MotorB (the 4pin terminal could be direct-to-motor, but I opted for the board being the middle man). 

Also, the controller (should) be controlled through serial (USB).

## Outputs

The 12V, GND and 5V is directly outputted to the L298N driver board, as well as the:
 - Enable Pin A (MotorA's PWM Signal)
 - IN1 & IN2 Pins (MotorA's directional control)
 - Enable Pin B (MotorB's PWM Signal)
 - IN3 & IN4 Pins (MotorB's directional control)

The board also takes the 12V GND 5V input from the L298N and outputs it to the motor through the bottom 2x05 pin headers after getting the output from the L298N.
