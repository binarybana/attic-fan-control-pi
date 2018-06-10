## Attic fan control

Need to have a particle photon where you can read temperature like so:
```
curl "https://api.particle.io/v1/devices/$ATTIC_DEVICE_ID/temp?access_token=$ATTIC_ACCESS_TOKEN"
```

Also, if cross compiling to ARM using `cross` then you'll need to run with `SSL_CERT_DIR=/etc/ssl/certs ./pi-gpio` on the target platform. (See [this issue](https://github.com/japaric/cross/issues/119)).

### TODO
  - [x] - First figure out which GPIO pins control which rooms
  - [x] - Get temp data from particle (either particle cloud function or GCE datastore query?)
  - [x] - Some exponential scaling to account for poor air mixing and sensor location
  - [x] - Thermostat behavior
  - [x] - Hook up thermostat setpoint and behavior to REST API
    - [x] - Switch to Gotham, Rouille, or Rocket from simple server
  - [ ] - Clean up deployment, add systemd unit file, add docs for deployment
  - [ ] - Call out to weather service for outside temp/humidity
  - [ ] - Add logic to disable thermostat when outside weather is bad
  - [ ] - Add logic to control nest thermostat instead

### Later
[ ] - Google voice activity integration
