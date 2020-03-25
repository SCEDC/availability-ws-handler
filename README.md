# Handlers for event and query endpoints of FDSN availability web service.

The FDSN availability web service has two endpoint; extent and query. Please see the [spec](https://fdsn.org/webservices/fdsnws-availability-1.0.pdf) for details.  

Each of these end points have separate code that handle their requests. The code receive GET or POST requests, query database tables and write output in appropriate format. They also sanitize and validate the input to prevent XSS attacks.  

The handlers are written in [rust](https://www.rust-lang.org/).  
  

## Dependencies  

  * Oracle instantclient 11.2 and up
  * Set LD_LIBRARY_PATH to instantclient install location.


## How to compile  

  * Install the rust compiler, rustc, and package builder, cargo.
  * Run **cargo build** to build a debug version of the executabl. **cargo build --release** will build a release version.
  

## Extent endpoint

The handler for extent endpoint is called **extent**. 

The extent endpoint serves continuous as well as triggered data, use the --asset to make your choice.  

The extent endpoint supports output formats of text, geocsv (spec [here](http://geows.ds.iris.edu/documents/GeoCSV.pdf)) and json. 

```
./extent --help
Availaibility handler 1.0
Reports availability of cont & trig waveforms

USAGE:
    extent [FLAGS] [OPTIONS]
	
FLAGS:
      --STDIN      POST input
      --debug      
      -h, --help       Prints help information
      -V, --version    Prints version information
							
OPTIONS:
      --asset <asset>             type of asset. acceptable values are [continuous, triggered]
      --config <cfg>              config file. Please see README to learn what is expected from the config file.
      --cha <chan>                channel code. wildcards supported at * and ?
      --endtime <end_time>        end date and time of request. format is YYYY-MM-DDTHH:MM:SS[.SSSSSS]
      --format <format>           output format = text|geocsv|json
      --loc <loc>                 location code. wildcards supported at * and ?
      --net <net>                 network code. wildcards supported at * and ?
      --sta <sta>                 station code. wildcards supported at * and ?
      --starttime <start_time>    start date and time of request. format is YYYY-MM-DDTHH:MM:SS[.SSSSSS]																									
```

*Some example usages*  
  
```  
./extent --asset triggered --net CI --sta BAK --cha BHZ 

./extent --asset continuous --net CI --sta BAK --cha B* --format json

./extent --asset continuous --net CI --sta BAK --cha B* --format geocsv --config /some/path/config.d
```
  
  
## Query endpoint

The handler for query endpoint is called **query**.  

The query endpoint only serves continuous data. It supports output formats of text, geocsv, json and request. 

```
./query --help  

Availaibility handler for query endpoint 1.0  
Reports availability of cont waveforms including gaps  

USAGE:
    query [FLAGS] [OPTIONS]

FLAGS:
        --STDIN      POST input
        --debug
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --config <cfg>              config file. Please see README to learn what is expected from the config file.
        --cha <chan>                channel code
		--endtime <end_time>        end date and time of request. format is yyyy-mm-ddThh:mm:ss[.ssssss]
		--format <format>           output format = text|json|geocsv|request
		--loc <loc>                 location code
		--net <net>                 network code
		--sta <sta>                 station code
		--starttime <start_time>    start date and time of request. format is yyyy-mm-ddThh:mm:ss[.ssssss]
```

*Some usage examples*  

```
./query --net CI --sta BAK --cha BHZ --starttime 2019-01-01T00:00:00 --endtime 2019-01-02T00:00:00
```


## Configuration files

**config.d** is the configuration file for extent and **config_query.d** is for query.  

The --cfg/--config option can be used to provide config file. But if none is provided, a default config file is used and must be present at the same location as the handler.

Each handler has it's own config file created in the parameter = value format, one on each line. Parameters included are  

**dbhost** 
Hostname of your database server  

**dbuser**  
Database user to connect to the database  

**dbpass**  
Password for dbuser  

**dbname**  
SID or name of the database to connect to  

**logginconfig**  
Filename with path of .yml file that contains the configuration for logging messages  

*A sample config.d*  

```
dbhost = www.some.hostname.com
dbpass = a-password
dbuser = some-user
dbname = some-db-name

loggingconfig = /app/wsmgr/wssconfig/run/availability/log4rs.yml
```
  
  
  
### Configuration file for logging    

The handlers use a rust crate called log4rs which is modelled after Java's log4j and Logback libraries. https://docs.rs/log4rs/0.11.0/log4rs/  

Each handler must have a separate logging config file, which is a .yml file. **This config file is to log messages output by the handler and is separate from the log4j.properties used by WSS.**   

A sample log4rs.yml, used by extent

```
refresh_rate: 30 seconds

appenders:
#And appender named "availability-handler" that writes to a file with a custom pattern encoder
  availability-handler:
    kind: rolling_file
    path: "/tmp/availability-handler.log"
    encoder:
      pattern: "{d} {I} - {m}{n}"
    policy:
      kind: compound
      trigger:
        kind: size
        limit: 500mb
      roller:
        kind: fixed_window
        base: 1
        count: 20
        pattern: "/tmp/availability-handler.{}.log"

#Set default logging level to "info" and attach the "stdout" appender to the root
root:
  level: info
  appenders:
    - availability-handler
```
