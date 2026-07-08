# Mover ROS2 Testbed (twenty-dollar project)

A small project to convert an old iRobot Essential into a ROS2 Testbed, with simple and cost-effective LiDAR-based navigation. No it isn't actually only twenty dollars, that's only the cost of the robot only, and because it rolls of the tongue nicely.

Structure:
 - `mover_controller`: closed-loop motor controller with a Pi Pico and the L298N (WIP)
 - `mover_pcb`: organised pcb for the pi pico, motors and motor-controller to interface nicely
 - `ros_ws`: workspace containing the packages & description for the robot.

## Parts (WIP)

| Part | Source | Cost (AUD) | Notes |
| ------------- | -------------- | -------------- | -------------- |
| Robot | Anywhere | $20 - $30 | |
| Raspberry Pi Pico | Anywhere | $5 - $10 | Should be the SMD version |
| PCB | JLCPCB | ~$11 | Includes shipping for 5pcs. Also includes the monthly deal |
| Terminal Blocks & Header pins | Jaycar | ~$12| 2x03 and 1x04 Terminal blocks and standard 40x headers to clip to size |
| Resistors (47k, 10k) | Jaycar | <$2 | |
| L298N | Jaycar / AliExpress |  $5 - $20 | |
| Raspberry Pi / Mini PC | Anywhere | $100 - $250 | Probably the most expensive part, if you replace the Pi Pico with a wireless one, it is possible to skip this and not use ROS |
| LiDAR Module (WitMotion D6) | AliExpress | $50 - $150 | Another really expensive part, this one could also be skipped if manual control via Pi Pico is preferred |
| Wires and Connectors | Anywhere | ~$20 | Really depends on how much soldering vs finding connectors. I opted to basically be completely non-intrusive and use stock connectors from the robot (JST ZH motor connectors) |
