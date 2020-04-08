extern crate clap;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate regex;
extern crate hostname;
extern crate chrono;
extern crate oracle;
#[macro_use]
extern crate json;
extern crate availability_handler;

use std::env;
use clap::{Arg, App};
use regex::Regex;
use std::process::exit;
use std::error::Error;
use std::collections::BTreeMap;
use chrono::prelude::*;
use oracle::Connection;
//to read POST input
use std::io; 
use std::io::prelude::*;


use availability_handler::{QueryParam, Output, Settings, create_sql, write_headings, parse_configuration, format_datetime, pad_datetime};


fn main() {

    let mut cfg = Settings {
        dbhost: String::from(""),
        dbpass: String::from(""),
        dbuser: String::from(""),
        dbname: String::from(""),
        logfileconfig: String::from("")
    };

    let matches = App::new("Availaibility handler for query endpoint")
        .version("1.0")
        .about("Reports availability of cont waveforms including gaps")
        .arg(Arg::with_name("cfg")
            .long("config")
            .value_name("cfg")
            .help("config file. Please see README to learn what is expected from the config file.")
            .takes_value(true))
        .arg(Arg::with_name("net")
            .long("net")
            .value_name("net")
            .help("network code")
            .takes_value(true))
        .arg(Arg::with_name("sta")
            .long("sta")
            .value_name("sta")
            .help("station code")
            .takes_value(true))
        .arg(Arg::with_name("cha")
            .long("cha")
            .value_name("chan")
            .help("channel code")
            .takes_value(true))
        .arg(Arg::with_name("loc")
            .long("loc")
            .value_name("loc")
            .help("location code")
            .allow_hyphen_values(true) 
            .takes_value(true))
	.arg(Arg::with_name("starttime")
	    .long("starttime")
  	    .value_name("start_time")
	    .help("start date and time of request. format is yyyy-mm-ddThh:mm:ss[.sssss]")
	    .takes_value(true))
	.arg(Arg::with_name("endtime")
	    .long("endtime")
	    .value_name("end_time")
	    .help("end date and time of request. format is yyyy-mm-ddThh:mm:ss[.sssss]")
	    .takes_value(true))
        .arg(Arg::with_name("format")
            .long("format")
            .value_name("format")
            .help("output format = text|json|geocsv|request")
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


    let mut cfgfile = env::current_exe().unwrap().parent().unwrap().join("config_query.d").to_str().unwrap().to_string();

    if matches.value_of("cfg") != None {
        cfgfile = String::from(matches.value_of("cfg").unwrap());
    }
    parse_configuration(cfgfile, &mut cfg);
    
    log4rs::init_file(&cfg.logfileconfig.clone(), Default::default()).unwrap();

    info!("----- NEW REQUEST ------");

    match env::var_os("USERAGENT") {
        Some(val) => info!("USERAGENT = {:?}", val.into_string().unwrap()),
        None => info!("USERAGENT not defined")
    }
    match env::var_os("IPADDRESS") {
        Some(val) => info!("IPADDRESS = {:?}", val.into_string().unwrap()),
        None => info!("IPADDRESS not defined")
    }


    let mut format: String = String::from("text");
    let mut starttime: String = String::from("");
    let mut endtime: String = String::from("");
    let mut starttime_global_input: String = String::from("");
    let mut endtime_global_input: String = String::from("");
    let mut debug = false;
    let mut list_of_query_params: Vec<QueryParam> = Vec::new();
    if matches.is_present("STDIN") { //POST request input        
            
        let stdin = io::stdin();

        for line in stdin.lock().lines() {
            let input_line = line.unwrap();
       
            info!("for query {:?}, len = {}", input_line, input_line.len());
               
            if input_line.len() == 0 { //ignore empty lines
                continue;
            }
            let mut temp_vec: Vec<&str>;        
            info!("{:?}", input_line);
            info!("is & present in input_line {:?}", input_line.find('&'));
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
                        asset:  String::from("")
                    };
                
                    info!("{:?}", query_param);
                    list_of_query_params.push(query_param);
                }
            }
        }     
            
    } else { //GET request input

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
        if matches.value_of("format") != None {
           format = String::from(matches.value_of("format").unwrap());
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

        if matches.is_present("debug"){	    
            println! ("{}","debug is turned on");
            debug = true;
        }
        
        list_of_query_params.push(query_param);
    }    
    
    info!("{:?}", list_of_query_params);
    info!("{}", debug);
    info!("validate_request");
    
    if validate_request(&list_of_query_params) > 0 { 
        exit(3); 
    }
    
    info!("handle_request");
    if handle_request(&list_of_query_params, format, &cfg, debug) > 0 { //handle_request returns 3 in case of errors
        exit(3);
    }
} //end of main


// Exit status = description                                                                                         
//                                                                                                                   
// 1 = General error                                                                                                 
// 2 = No data. Request was successful but no data was retrieved                                                     
// 3 = Invalid argument/parameter                                                                                    
// 4 = too much data requested                                                                                       
// 0 = Success  

fn handle_request(query_params: &Vec<QueryParam>, format: String, cfg: &Settings, debug: bool) -> i32 {
    info!("handle_request");

    let mut dbconnstring: String;
    let mut sql: String;

    dbconnstring = cfg.dbhost.clone();
    dbconnstring.push_str("/");
    dbconnstring.push_str(&cfg.dbname.clone());

    info!("Connecting to db {}", dbconnstring);
    let conn = Connection::connect(&cfg.dbuser, &cfg.dbpass, &dbconnstring, &[]).unwrap();  

    let mut output: BTreeMap<String, Vec<Output>> = BTreeMap::new();

    //OUTPUT HEADINGS    
    info!("write_headings");
    write_headings(&format);
    for param in query_params {
        sql = String::from("select net, sta, seedchan, location, samplerate, datetime_on, datetime_off, truetime.nominal2stringf(datetime_on) || '00', truetime.nominal2stringf(datetime_off) || '00' "); 
        info!("{:?}", &param);
        
        info!("create_sql");
        create_sql(String::from("query_availability"), param, &mut sql);
        
        info!("{}", sql);
        if debug {
            println!("{}", sql);
        }
    
        let mut stmt = conn.prepare(&sql, &[]).unwrap();
        let rows = stmt.query(&[]).unwrap();
    
        let mut net: String;
        let mut sta: String;
        let mut chan: String;
        let mut location: String;
        let mut start: f64;
        let mut start_iso: String;
        let mut starttime: String = "".to_string();
        let mut end: f64;
        let mut end_iso: String;
        let mut endtime: String = "".to_string();
        let mut rate: f32;
        
        let mut sncl;
        let mut prev_sncl = "".to_string();
        let mut user_input_start: f64 = -1.0;
        let mut user_input_end: f64 = -1.0;
    
        if !param.starttime.is_empty() {
            starttime = param.starttime.clone();
            if Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+$").unwrap().is_match(&param.starttime) {
                starttime.push_str(" +0000"); //add +0000 as DateTime::parse_from_str required timezone information  
            } else if Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$").unwrap().is_match(&param.starttime) {
                    starttime.push_str(".000000 +0000"); //add +0000 as DateTime::parse_from_str required timezone information
            } else if Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap().is_match(&starttime) {
                    starttime.push_str("T00:00:00.000000 +0000");
            }            
            
            user_input_start = match DateTime::parse_from_str(&starttime,"%Y-%m-%dT%H:%M:%S.%f %z") {
                Ok(v) => v.timestamp() as f64,
                Err(e) => { println!("Icky error for starttime  {:?} {:?}", e, e.description());
                            -1.0
                }
            };
        }

        if !param.endtime.is_empty() {
            endtime = param.endtime.clone();
            
            if Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+$").unwrap().is_match(&param.endtime) { 
                endtime.push_str(" +0000"); //add +0000 as DateTime::parse_from_str required timezone information
            } else if Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$").unwrap().is_match(&param.endtime) {
                endtime.push_str(".00000 +0000"); //add +0000 as DateTime::parse_from_str required timezone information
            } else if Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap().is_match(&param.endtime) {
                endtime.push_str("T00:00:00.00000 +0000");
            }
            
            //println!("endtime = {}", endtime);
            user_input_end = match DateTime::parse_from_str(&endtime,"%Y-%m-%dT%H:%M:%S.%f %z") {
                Ok(v) => v.timestamp() as f64,
                Err(e) => { println!("Icky error for endtime {:?} {:?}", e, e.description());
                            -1.0
                }
            };
           
        }
       
        info!("process result set");
        for row_result in &rows {

            let row = row_result.unwrap();        
            net = row.get(0).unwrap();
            sta = row.get(1).unwrap();
            chan = row.get(2).unwrap();
            location = row.get(3).unwrap();
            start = row.get(5).unwrap();
            end = row.get(6).unwrap();
            start_iso = row.get(7).unwrap();
            start_iso = start_iso.replace("/","-").replace(" ","T");
            start_iso.push_str("Z");
            end_iso = row.get(8).unwrap();
            end_iso = end_iso.replace("/","-").replace(" ","T");
            end_iso.push_str("Z");
            rate = row.get(4).unwrap();

            //let's start to stitch together records
            
            sncl = format!("{}.{}.{}.{}", net, sta, chan, location);
            
            if !output.contains_key(&sncl) { //initialize
                //if user specified start is between first record's start and end, start output with user input value
                if user_input_start > start && user_input_start < end && !param.starttime.is_empty() {
                    start_iso = pad_datetime(&format_datetime(&param.starttime.clone()));
                }
                let temp = Output {
                    net: net,
                    sta: sta,
                    chan: chan,
                    loc: location,
                    sr: rate,
                    start: start,
                    end: end,
                    start_iso: start_iso,
                    end_iso: end_iso
                };
                output.insert(sncl, vec![temp]);
                //if user specified end is not a day boundary, update the end date of last record for the sncl to match user specified end
                if !prev_sncl.is_empty() && !param.endtime.is_empty() { 
                    if user_input_end < output.get(&prev_sncl).unwrap().last().unwrap().end {
                        output.get_mut(&prev_sncl).unwrap().last_mut().unwrap().end_iso = pad_datetime(&param.endtime.clone());
                    }
                }
                continue;
            }
        if output.get(&sncl).unwrap().len() > 0 {
            if start - output.get_mut(&sncl).unwrap().last_mut().unwrap().end == 1.0 {
                output.get_mut(&sncl).unwrap().last_mut().unwrap().end = end;
                output.get_mut(&sncl).unwrap().last_mut().unwrap().end_iso = end_iso;
            } else {
                output.get_mut(&sncl).unwrap().push(Output{
                    net: net,
                    sta: sta,
                    chan: chan,
                    loc: location,
                    sr: rate,
                    start: start,
                    end: end,
                    start_iso: start_iso,
                    end_iso: end_iso
                })
            }           
        }
        prev_sncl = sncl;
        } // end of row_result in &rows {
        
        //if user specified end is not a day boundary, update the end date of last record for the sncl to match user specified end
        if !prev_sncl.is_empty() && !param.endtime.is_empty() {
            if user_input_end < output.get(&prev_sncl).unwrap().last().unwrap().end {
                output.get_mut(&prev_sncl).unwrap().last_mut().unwrap().end_iso = pad_datetime(&param.endtime.clone());
            }
        }
        //info!("OUTPut {:?}", output);
    } //end of for param in query_params

    //for json output
    let mut loop_count = 0;
    let mut json_data = object!{
        "created" => Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "version" => "1.0",
        "asset_name" => "continuous",
    };

    info!("write output in {} format", format);
    //info!("output is {:?}", output);
    for key in output.keys() {
        for item in output[key].iter() {
            //info! ("{:?}", item);
            if format == "text" {
                println! ("{n:<width$} {s:<widths$} {l:<widthl$} {c:<widthc$} {q:widthq$} {r:<widthr$.2} {earliest} {latest} NA NA OPEN",
                          n=item.net,width=2,
                          s=item.sta,widths=item.sta.len(),
                          l=item.loc.replace(" ","-"), widthl=2,
                          c=item.chan, widthc=3,
                          q=" ", widthq=1,
                          r=item.sr,widthr=6,
                          earliest=item.start_iso,
                          latest=item.end_iso);

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
                    "latest" => item.end_iso.clone(),
                    "updated" => "NA",
                    "timespans" => "NA",
                    "restriction" => "OPEN"
                };
                loop_count = loop_count + 1
            } else if format =="request" {
                println!("{n:<width$} {s:<widths$} {l:<widthl$} {c:<widthc$} {earliest} {latest}",
                         n=item.net,width=2,
                         s=item.sta,widths=item.sta.len(),
                         l=item.loc.replace(" ","-"), widthl=2,
                         c=item.chan, widthc=3,
                         earliest=item.start_iso,
                         latest=item.end_iso);
            }
        }        
    }
    if format == "json" {
        println!("{}", json_data.dump());
    }
    return 0
    
} //end of handler


fn validate_request(query_params: &Vec<QueryParam>) -> i32 {
    info!("validate request");
    let mut re;
    for param in query_params {
        if !param.net.is_empty() {
            re = Regex::new(r"^[a-zA-Z0-9\*\?]{2}$").unwrap();
            //let net = String::from(param.net);
            let spl : Vec<&str> = param.net.split(',').collect();
            for item in spl.iter(){
                if !re.is_match(&item) {
                eprintln! ("{} {} {}", "Unrecognized network code ", item, ". Valid format is [1-2 characters].");
	            return 3;
	        }               
            }
        }
    
        if !param.sta.is_empty() {            
            re = Regex::new(r"^[A-Za-z0-9\*\?%]{1,5}$").unwrap();
            let spl: Vec<&str> = param.sta.split(',').collect();
            for item in spl.iter() {
	        if !re.is_match(&item) {
	            eprintln! ("{} {} {}", "Unrecognized station code ", item, ". Valid format is [1-5 characters].");
	            return 3;
	        }
            }
        }
        
        if !param.chan.is_empty() {           
            re = Regex::new(r"^[A-Za-z0-9\*\?%]{1,3}$").unwrap();
            let spl: Vec<&str> = param.chan.split(',').collect();
            for item in spl.iter() {
	        if !re.is_match(&item) {
	            eprintln! ("{} {} {}", "Unrecognized channel code ", item, ". Valid format is [1-3 characters].");
	            return 3;
	        }
            }
        }

        if !param.loc.is_empty() {           
            re = Regex::new(r"^[A-Za-z0-9\-\*\?%]{1,2}$").unwrap();
            let spl: Vec<&str> = param.loc.split(',').collect();
            for item in spl.iter() {
	        if !re.is_match(&item) {
	            eprintln! ("{} {} {}", "Unrecognized location code ", item, ". Valid format is [1-2 characters].");
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
            if param.format != "text" && param.format != "geocsv" && param.format != "json" && param.format != "request" {
                eprintln!("{} {} {}", "Unrecognized format value ", param.format, "Valid format values are [text, geocsv, json, request].");
                return 3;
            }
        }
    }
    return 0;
}
