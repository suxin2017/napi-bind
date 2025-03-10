const m = require('./index');

process.nextTick(() => {
    let w = new m.BindingWatcher();
    w.start(
        (data) => {
            console.log(data)
        }
    )
    console.log("bind envent=======")

    // only show "async call" ,don't show "async no lock"
    console.log(w.start)
    w.loopSpawn(
        (data) => {
            console.log(data)
        }
    )
    console.log("end envent=======")

    // same

}
);