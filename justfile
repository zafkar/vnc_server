xserver: kill
    Xephyr -screen 640x480 :2 &
    sleep 1 &

random_app: xserver
    DISPLAY=:2 mousepad &

run: xserver
    DISPLAY=:2 cargo run

test: random_app run

kill:
    killall mousepad || true
    killall Xephyr || true
    killall vnc_server || true