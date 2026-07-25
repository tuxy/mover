import machine
import time

enc_a = machine.Pin(8, machine.Pin.IN, machine.Pin.PULL_UP)
enc_b = machine.Pin(9, machine.Pin.IN, machine.Pin.PULL_UP)

print("Reading encoder pins 100x/s — rotate the encoder now")
print("a b")

last_a = enc_a.value()
last_b = enc_b.value()

for _ in range(200):
    va = enc_a.value()
    vb = enc_b.value()
    if va != last_a or vb != last_b:
        print("{} {}  <-- CHANGED".format(va, vb))
        last_a = va
        last_b = vb
    time.sleep_ms(10)

print("\nDone. Final levels: a={}, b={}".format(enc_a.value(), enc_b.value()))
