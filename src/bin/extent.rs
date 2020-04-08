extern crate clap;
extern crate oracle;
extern crate chrono;
extern crate regex;
extern crate hostname;
#[macro_use]
extern crate log;
extern crate log4rs;
#[macro_use]
extern crate json;
extern crate availability_handler;

use std::str;
use clap::{Arg, App};
use oracle::Connection;
use std::collections::BTreeMap;
use std::process::exit;
use regex::Regex;
use std::env;
use chrono::prelude::*;
//to read POST input
use std::io; 
use std::io::prelude::*;
use availability_handler::{QueryParam, ExtentOutput,  Settings, create_sql, write_headings, parse_configuration, format_datetime};


fn main() {

    let mut cfg = Settings {
        dbhost: String::from(""),
        dbpass: String::from(""),
        dbuser: String::from(""),
        dbname: String::from(""),
        logfileconfig: String::from("")
    };


    let matches = App::new("Availaibility handler")
        .version("1.0")
        .about("Reports availability of cont & trig waveforms")
        .arg(Arg::with_name("cfg")
            .long("config")
            .value_name("cfg")
            .help("config file. Please see README to learn what is expected from the config file.")
            .takes_value(true))
        .arg(Arg::with_name("net")
            .long("net")
            .value_name("net")
            .help("network code. wildcards supported at * and ?")
            .takes_value(true))
        .arg(Arg::with_name("sta")
            .long("sta")
            .value_name("sta")
            .help("station code. wildcards supported at * and ?")
            .takes_value(true))
        .arg(Arg::with_name("cha")
            .long("cha")
            .value_name("chan")
            .help("channel code. wildcards supported at * and ?")
            .takes_value(true))
        .arg(Arg::with_name("loc")
            .long("loc")
            .value_name("loc")
            .allow_hyphen_values(true)
            .help("location code. wildcards supported at * and ?")
            .takes_value(true))
	.arg(Arg::with_name("starttime")
	    .long("starttime")
  	    .value_name("start_time")
	    .help("start date and time of request. format is YYYY-MM-DDTHH:MM:SS[.SSSSSS]")
	    .takes_value(true))
	.arg(Arg::with_name("endtime")
	    .long("endtime")
	    .value_name("end_time")
	    .help("end date and time of request. format is YYYY-MM-DDTHH:MM:SS[.SSSSSS]")
	    .takes_value(true))
	.arg(Arg::with_name("asset")
	    .long("asset")
	    .value_name("asset")
            .help("type of asset. acceptable values are [continuous, triggered]")
	    .takes_value(true))
        .arg(Arg::with_name("format")
            .long("format")
            .value_name("format")
            .help("output format = text|geocsv|json")
            .takes_value(true))
        .arg(Arg::with_name("STDIN")
            .long("STDIN")
            .value_name("STDIN")
            .help("POST input")
            .takes_value(false))
	.arg(Arg::with_name("debug")
	    .long("debug")
	    .value_name("debug")
	    .takes_value(false))
        .get_matches();


    //println!("{}", env::current_exe().unwrap().parent().unwrap().join("config.d").to_str().unwrap().to_string());
    
    let mut cfgfile = env::current_exe().unwrap().parent().unwrap().join("config.d").to_str().unwrap().to_string();
        
    if matches.value_of("cfg") != None {
        cfgfile = String::from(matches.value_of("cfg").unwrap());
    }
    
    parse_configuration(cfgfile, &mut cfg);
        
    log4rs::init_file(&cfg.logfileconfig, Default::default()).unwrap();

    info!("----- NEW REQUEST ------");
    match env::var_os("USERAGENT") {
        Some(val) => info!("USERAGENT = {:?}", val.into_string().unwrap()),
        None => info!("USERAGENT not defined")
    }
    match env::var_os("IPADDRESS") {
        Some(val) => info!("IPADDRESS = {:?}", val.into_string().unwrap()),
        None => info!("IPADDRESS not defined")
    }


    //A vector containing a hashmap. Each hashmap is made of one set of input parameters
    //[{net:CI, sta:AGO, starttime:2010-01-01T00:00:00, endtime:2010-12-01T00:00:00},
    // {net:CI, sta:WBS, starttime:2018-01-01T00:00:00, endtime:2018-12-01T00:00:00},..{}]
    let mut starttime: String;
    let mut endtime: String;
    let mut starttime_global_input: String = String::from("");
    let mut endtime_global_input: String = String::from("");
    let mut list_of_query_params: Vec<QueryParam> = Vec::new();
    let mut format: String = String::from("text");    
    let mut asset: String = String::from("continuous");
    let mut debug = false;

    //POST request
    if matches.is_present("STDIN"){

        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let input_line = line.unwrap();

            info!("{:?}", input_line);

            if input_line.len() == 0 { //ignore empty lines                                          
                continue;
            }
            let mut temp_vec: Vec<&str>;
            if input_line.find('&') != None { //post data was sent in the form of net=NN&sta=SSSSS&loc=LL&cha=CCC             
                info!("--post-data input"); 

                temp_vec = input_line.split('&').collect();
                info!("temp_vec = {:?}", temp_vec);
                let mut query_param = QueryParam {
                    net: String::from(""),
                    sta: String::from(""),
                    chan: String::from(""),
                    loc: String::from(""),
                    starttime: String::from(""),
                    endtime: String::from(""),
                    format: String::from("text"),
                    asset:  String::from("")
                };
                for item in temp_vec {
                    info!("item = {} ", item);
                    let mut temp: Vec<&str>;                    
                    if item.find("net") != None || item.find("network") != None {
                        temp = item.split('=').collect();
                        query_param.net = String::from(temp[1])
                    }
                    if item.find("sta=") != None || item.find("station") != None {
                        temp = item.split('=').collect();
                        query_param.sta = String::from(temp[1])
                    }
                    if item.find("cha") != None || item.find("channel") != None {
                        temp = item.split('=').collect();
                        query_param.chan = String::from(temp[1])
                    }
                    if item.find("loc") != None || item.find("location") != None {
                        temp = item.split('=').collect();
                        query_param.loc = String::from(temp[1])
                    }
                    if item.find("start") != None || item.find("starttime") != None {
                        temp = item.split('=').collect();
                        query_param.starttime = format_datetime(&String::from(temp[1]));
                    }
                    if item.find("end") != None || item.find("endtime") != None {
                        temp = item.split('=').collect();
                        query_param.endtime = format_datetime(&String::from(temp[1]));
                    }
                    if item.find("format") != None {
                        temp = item.split('=').collect();
                        format = String::from(temp[1]);                            
                    }
                }
                info!("{:?}", query_param);
                list_of_query_params.push(query_param);
            } else {
                if input_line.find("format") != None {
                    temp_vec = input_line.split("=").collect();
                    format = String::from(temp_vec[1]);
                } else if input_line.find("starttime") != None {
                    temp_vec = input_line.split("=").collect();
                    starttime_global_input = format_datetime(&String::from(temp_vec[1]));
                } else if input_line.find("endtime") != None {
                    temp_vec = input_line.split("=").collect();
                    endtime_global_input = format_datetime(&String::from(temp_vec[1]));
                } else {
                    let params: Vec<&str> = input_line.split(" ").collect();
                    
                    info!("{:?}, len of params = {}", params, params.len());
                    if params.len() == 6 {
                        starttime = format_datetime(&String::from(params[4]));
                        endtime = format_datetime(&String::from(params[5]));
                    } else {
                        starttime = starttime_global_input.clone();
                        endtime = endtime_global_input.clone();
                    }
                    let mut query_param = QueryParam {
                        net: String::from(params[0]),
                        sta: String::from(params[1]),
                        chan: String::from(params[3]),
                        loc: String::from(params[2]),
                        starttime: starttime.clone(),
                        endtime: endtime.clone(),
                        format: format.clone(),
                        asset:  String::from("continuous")
                    };

                    info!("{:?}", query_param);
                    list_of_query_params.push(query_param);
                }
            }
        }
        
    } else { //end of STDIN input       

        let mut query_param = QueryParam {
            net: String::from(""),
            sta: String::from(""),
            chan: String::from(""),
            loc: String::from(""),
            starttime: String::from(""),
            endtime: String::from(""),
            format: String::from("text"),
            asset:  String::from("continuous")
        };
        if matches.value_of("format") != None {
            format = String::from(matches.value_of("format").unwrap());
            query_param.format = String::from(matches.value_of("format").unwrap());
        }

        if matches.is_present("debug"){	    
            println! ("{}","debug is turned on");
            debug = true;
        }
        

        if matches.value_of("net") != None {
            query_param.net = String::from(matches.value_of("net").unwrap());
        }

        if matches.value_of("sta") != None {
            query_param.sta = String::from(matches.value_of("sta").unwrap());
        }

        if matches.value_of("cha") != None {
            query_param.chan= String::from(matches.value_of("cha").unwrap());
        }

            
        if matches.value_of("loc") != None {
            query_param.loc = String::from(matches.value_of("loc").unwrap());
        }

        if matches.value_of("starttime") != None {
            query_param.starttime = format_datetime(&String::from(matches.value_of("starttime").unwrap()));
        }        
             
        if matches.value_of("endtime") != None {
            query_param.endtime = format_datetime(&String::from(matches.value_of("endtime").unwrap()));
        }

        if matches.value_of("asset") != None {
            asset = String::from(matches.value_of("asset").unwrap_or("continuous"));
            query_param.asset = String::from(matches.value_of("asset").unwrap_or("continuous"));
        } 
                      
        if debug {
            println!("query_param {:?}", query_param);
        } 
            
        list_of_query_params.push(query_param);
    }
        

    if validate_request(&list_of_query_params) > 0 {
       exit(3); //validate_request always returns 3 in case of error
    }
    info!("handle_request");
    if handle_request(&list_of_query_params, format, asset, &cfg, debug) > 0 { //handle_request returns 3 in case of errors                                                                                           
        exit(3);
    }
}


// Exit status = description                                                                                         
//                                                                                                                   
// 1 = General error                                                                                                 
// 2 = No data. Request was successful but no data was retrieved                                                     
// 3 = Invalid argument/parameter                                                                                    
// 4 = too much data requested                                                                                       
// 0 = Success 

fn handle_request(query_params: &Vec<QueryParam>, format: String, asset: String, cfg: &Settings, debug: bool) -> i32 {
    let mut dbconnstring: String;
    let mut sql: String;
    let mut output: BTreeMap<String, Vec<ExtentOutput>> = BTreeMap::new();   

    //read database connection info from file                                                                             
    dbconnstring = cfg.dbhost.clone();
    dbconnstring.push_str("/");
    dbconnstring.push_str(&cfg.dbname.clone());

    info!("Connecting to db {}", dbconnstring);
    let conn = Connection::connect(&cfg.dbuser,&cfg.dbpass, &dbconnstring, &[]).unwrap();

    //OUTPUT HEADINGS                                                                                
    info!("write_headings");
    write_headings(&format);

    //create sql
    for param in query_params {
      sql = String::from("select net, sta, seedchan, location, samplerate, truetime.nominal2stringf(datetime_on) || '00', truetime.nominal2stringf(datetime_off) || '00' ");
      
        create_sql(String::from("availability"), param, &mut sql);

        info!("{}", sql);
        if debug {
            println!("{}", sql);
        }
    
        //info!("Connecting to db {}", dbconnstring);
        //let conn = Connection::connect(&cfg.dbuser,&cfg.dbpass, &dbconnstring, &[]).unwrap();
        let mut stmt = conn.prepare(&sql, &[]).unwrap();
        info!("Running query");
        let rows = stmt.query(&[]).unwrap();
        
        let mut net: String;
        let mut sta: String;
        let mut chan: String;
        let mut location: String;
        let mut start_iso: String;
        let mut end_iso: String;
        let mut rate: f32; 
        let mut sncl;

        info!("writing results");
        for row_result in &rows {
            let row = row_result.unwrap();
            net = row.get(0).unwrap();
            sta = row.get(1).unwrap();
            chan = row.get(2).unwrap();
            location = row.get(3).unwrap();
            start_iso = row.get(5).unwrap();
            start_iso = start_iso.replace("/","-").replace(" ","T");
            start_iso.push_str("Z");
            end_iso = row.get(6).unwrap();
            end_iso = end_iso.replace("/","-").replace(" ","T");
            end_iso.push_str("Z");
            rate = row.get(4).unwrap();
            

            sncl = format!("{}.{}.{}.{}.{}", net, sta, chan, location, rate);
            
            let temp = ExtentOutput {
                net: net,
                sta: sta,
                chan: chan,
                loc: location,
                sr: rate,
                start_iso: start_iso,
                end_iso: end_iso
            };
            output.insert(sncl, vec![temp]);
        } //end of for
        info!("OUTPUT {:?}", output);
    } //end of for param in query_params  


    let mut loop_count = 0;
    let mut json_data = object!{
        "created" => Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "version" => "1.0",
        "asset_type" => asset,
    };

    info!("write output in {} format", format);
    info!("output is {:?}", output);

    //display output             
    for key in output.keys() {
        for item in output[key].iter() {
            //info! ("{:?}", item);
            if format == "text" {

                println!("{n:<width$} {s:<widths$} {l:<widthl$} {c:<widthc$} {q:widthq$} {r:<widthr$.2} {earliest} {latest} {updated} {timespans} {restrictions}",
                         n=item.net,width=2,
                         s=item.sta,widths=item.sta.len(),
                         l=item.loc.replace(" ","-"), widthl=2,
                         c=item.chan, widthc=3,
                         q=" ", widthq=1,
                         r=item.sr,widthr=6,
                         earliest=item.start_iso,
                         latest=item.end_iso,
                         updated="NA",
                         timespans="NA",
                         restrictions="OPEN"
                );
            } else if format == "geocsv" {

                println!("{n:<width$}|{s:<widths$}|{l:<widthl$}|{c:<widthc$}|{q:widthq$}|{r:<widthr$.2}|{earliest}|{latest}|NA|NA|OPEN",
                         n=item.net,width=2,
                         s=item.sta,widths=item.sta.len(),
                         l=item.loc.replace(" ",""), widthl=item.loc.replace(" ","").len(),
                         c=item.chan, widthc=3,
                         q=" ", widthq=1,
                         r=item.sr,widthr=item.sr.to_string().len(),
                         earliest=item.start_iso,
                         latest=item.end_iso);
            } else if format == "json" {
                //build the json object
                json_data["datasources"][loop_count] = object!{
                    "network" => item.net.clone(),
                    "station" => item.sta.clone(),
                    "location" => item.loc.clone().replace(" ",""),
                    "channel" => item.chan.clone(),
                    "quality" => "",
                    "samplerate" => item.sr.clone(),
                    "earliest" => item.start_iso.clone(),
                    "latest" =>item.end_iso.clone(),
                    "timespans"=>"NA",
                    "updated"=>"NA",
                    "restriction"=>"OPEN"
                };
                loop_count = loop_count + 1;
            } 
        }
    } //end of for key in output.keys()  

    if format == "json" {
        println!("{}",json_data.dump());
    }
    return 0
}// end handle_request


fn validate_request(query_params: &Vec<QueryParam>) -> i32 {
    info!("validate request");
    let mut re;
    for param in query_params {
        if !param.net.is_empty() {
            re = Regex::new(r"^[a-zA-Z0-9\*\?]{2}$").unwrap();
            let spl : Vec<&str> = param.net.split(',').collect();
            for item in spl.iter(){
                if !re.is_match(&item) {
                eprintln! ("{} {} {}", "Unrecognized network code ", item, ". Valid format is [1-2 c\
haracters].");
                    return 3;
                    }
            }
        }

        if !param.sta.is_empty() {
            re = Regex::new(r"^[A-Za-z0-9\*\?%]{1,5}$").unwrap();
            let spl: Vec<&str> = param.sta.split(',').collect();
            for item in spl.iter() {
                if !re.is_match(&item) {
                    eprintln! ("{} {} {}", "Unrecognized station code ", item, ". Valid format is [1\
-5 characters].");
                    return 3;
                }
            }
        }

        if !param.chan.is_empty() {
            re = Regex::new(r"^[A-Za-z0-9\*\?%]{1,3}$").unwrap();
            let spl: Vec<&str> = param.chan.split(',').collect();
            for item in spl.iter() {
                if !re.is_match(&item) {
                    eprintln! ("{} {} {}", "Unrecognized channel code ", item, ". Valid format is [1\
-3 characters].");
                    return 3;
                }
            }
        }
        if !param.loc.is_empty() {
            re = Regex::new(r"^[A-Za-z0-9\-\*\?%]{1,2}$").unwrap();
            let spl: Vec<&str> = param.loc.split(',').collect();
            for item in spl.iter() {
                if !re.is_match(&item) {
                    eprintln! ("{} {} {}", "Unrecognized location code ", item, ". Valid format is [\
1-2 characters].");
                    return 3;
                }
            }
        }
        if !param.starttime.is_empty() {
            if !Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+$").unwrap().is_match(&param.starttime) &&
                !Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$").unwrap().is_match(&param.starttime) &&
                !Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap().is_match(&param.starttime){
                    eprintln! ("{} {} {}", "Unrecognized date time format ", param.starttime, ". Valid format is [YYYY-MM-DDTHH:MM:SS.SSSS].");
                    return 3;
                }
        }
        if !param.endtime.is_empty() {
            if !Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+$").unwrap().is_match(&param.endtime) &&
                !Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$").unwrap().is_match(&param.endtime) &&
                !Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap().is_match(&param.endtime){
                    eprintln! ("{} {} {}", "Unrecognized date time format ", param.endtime, ". Valid format is [YYYY-MM-DDTHH:MM:SS.SSSS].");
                    return 3;
                }
        }

        if !param.format.is_empty() {
            if param.format != "text" && param.format != "geocsv" && param.format != "json" {
                eprintln!("{} {} {}", "Unrecognized format value", param.format, ". Valid format values are [text,json,geocsv].");
                return 3;
            }
        }
    

        if !param.asset.is_empty() {
            if param.asset != "continuous" && param.asset != "triggered" {
                eprintln!("{} {} {}", "Unrecognized asset value ", param.asset, "Valid format values are [continuous, triggered].");
                return 3;
            }
        }
    }
    return 0;
}


