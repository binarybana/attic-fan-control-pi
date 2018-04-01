## Attic fan control

```
curl "https://api.particle.io/v1/devices/$ATTIC_DEVICE_ID/temp?access_token=$ATTIC_ACCESS_TOKEN"
```

### TODO
[x] - First figure out which GPIO pins control which rooms
[ ] - Get temp data from particle (either particle cloud function or GCE datastore query?)
[ ] - Some exponential scaling to account for poor air mixing and sensor location
[ ] - Thermostat behavior
[ ] - Hook up thermostat setpoint and behavior to REST API
  [ ] - Switch to Gotham, Rouille, or Rocket from simple server
[ ] - Clean up deployment, add systemd unit file, add docs for deployment

### Later
[ ] - Google voice activity integration
