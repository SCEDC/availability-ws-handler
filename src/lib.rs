extern crate config;

//to read POST input
use std::io; 
use std::io::prelude::*;
use std::error::Error;
use std::fs::File;
use std::path::Path;
//remove
use std::process::exit;

#[derive(Debug)]
pub struct QueryParam {
    pub net: String,
    pub sta: String,
    pub chan: String,
    pub loc: String,
    pub starttime: String,
    pub endtime: String,
    pub format: String,
    pub asset: String
}

#[derive(Debug)]
pub struct Output {
    pub net: String,
    pub sta: String,
    pub chan: String,
    pub loc: String,
    pub sr: f32,
    pub start: f64,
    pub end: f64,
    pub start_iso: String,
    pub end_iso: String
}

#[derive(Debug)]
pub struct ExtentOutput {
    pub net: String,
    pub sta: String,
    pub chan: String,
    pub loc: String,
    pub sr: f32,
    pub start_iso: String,
    pub end_iso: String
}

#[derive(Debug)]
pub struct Settings {
    pub dbhost: String,
    pub dbpass: String,
    pub dbuser: String,
    pub dbname: String,
    pub logfileconfig: String
}


pub fn parse_configuration(filename: String, config : &mut Settings) {
    
    let temp = filename.clone();
    // Open the path in read-only mode, returns `io::Result<File>`                                                        
    let mut file = match File::open(filename) {
        // The `description` method of `io::Error` returns a string that                                                                                                                  // describes the error                                                                         
        Err(why) => panic!("couldn't open file: {} {}. Use --config to provide a config file with path. ",temp, why.description()),
        Ok(file) => file,
    };
 
    
    let mut config_file_contents = String::new();
    file.read_to_string(&mut config_file_contents);

    let alllines: Vec<&str> =  config_file_contents.split("\n").collect();

    for line in alllines {
        if line.is_empty() {
            continue;
        }
        let v: Vec<&str> = line.trim().split("=").collect();
        if v[0].trim() == "dbhost" {
            config.dbhost = v[1].trim().replace("\"", "").to_string();
        }
        if v[0].trim() == "dbuser" {
            config.dbuser = v[1].trim().replace("\"", "").to_string();
        }
        if v[0].trim() == "dbpass" {
            config.dbpass = v[1].trim().replace("\"", "").to_string();
        }
        if v[0].trim() == "dbname" {
            config.dbname = v[1].trim().replace("\"", "").to_string();
        }
       if v[0].trim() == "loggingconfig" {
            config.logfileconfig = v[1].trim().replace("\"", "").to_string();
        }
    }
}


pub fn collect_post_input(list_of_query_params: &mut Vec<QueryParam>) {

    let mut format: String = String::from("text");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let input_line = line.unwrap();
        
        if input_line.len() == 0 { //ignore empty lines
            continue;
        }
        let mut temp_vec: Vec<&str>;        
        if input_line.find("format") != None {
            temp_vec = input_line.split("=").collect();
            format = String::from(temp_vec[1]);
        } 
        let params: Vec<&str> = input_line.split(" ").collect();

        //println!("{:?}", params);
        let mut query_param = QueryParam {
            net: String::from(params[0]),
            sta: String::from(params[1]),
            chan: String::from(params[3]),
            loc: String::from(params[2]),
            starttime: String::from(params[4]),
            endtime: String::from(params[5]),
            format: format.clone(),
            asset:  String::from("continuous")
        };
        list_of_query_params.push(query_param);
    }
    
}

pub fn create_sql(dbtable: String, query_param: &QueryParam, sql: &mut String) {
    let mut net_str: String = "".to_string();
    let mut sta_str: String = "".to_string();
    let mut chan_str: String = "".to_string();
    let mut loc_str: String = "".to_string();
    let mut starttime_str: String = "".to_string();
    let mut endtime_str: String = "".to_string();

    sql.push_str(" from ");
    sql.push_str(&dbtable);
    sql.push_str(" where datetime_on is not null and datetime_off is not null ");

    if !query_param.asset.is_empty() {
        if query_param.asset == "continuous" {
            sql.push_str(" and asset = 'C' ");
        }
        if query_param.asset =="triggered" {
            sql.push_str(" and asset = 'T' ");
        }
    }
    if !query_param.net.is_empty() {
        create_str_from_list(query_param.net.clone(), "net".to_string(), &mut net_str);
        append_to_sql(&net_str, sql);
    }
    if !query_param.sta.is_empty() {
        create_str_from_list(query_param.sta.clone(), "sta".to_string(), &mut sta_str);
        append_to_sql(&sta_str, sql);
    }
     if !query_param.chan.is_empty() {
         create_str_from_list(query_param.chan.clone(), "seedchan".to_string(), &mut chan_str);
         append_to_sql(&chan_str, sql);
    }
    if !query_param.loc.is_empty() {
        create_str_from_list(query_param.loc.clone().replace("--","  "), "location".to_string(), &mut loc_str);
        append_to_sql(&loc_str, sql);
    }

    
    if !query_param.starttime.is_empty() && !query_param.endtime.is_empty() {
         starttime_str = query_param.starttime.clone();                                                                                                                           
         starttime_str = starttime_str.replace("-","/").replace("T"," ");
         endtime_str = query_param.endtime.clone();                                                                                                                               
         endtime_str = endtime_str.replace("-","/").replace("T"," ");
         sql.push_str(" and (");
         sql.push_str(" ( datetime_on <= truetime.string2nominalf('");
         sql.push_str(&starttime_str);
         sql.push_str("') and datetime_off >= truetime.string2nominalf('");
         sql.push_str(&endtime_str);
         sql.push_str("') )");

         sql.push_str(" OR ");
         sql.push_str(" ( datetime_on between truetime.string2nominalf('");
         sql.push_str(&starttime_str);
         sql.push_str("') and truetime.string2nominalf('");
         sql.push_str(&endtime_str);
         sql.push_str("') ");

         sql.push_str(" OR datetime_off between truetime.string2nominalf('");
         sql.push_str(&starttime_str);
         sql.push_str("') and truetime.string2nominalf('");
         sql.push_str(&endtime_str);
         sql.push_str("') )");
         sql.push_str(" )");
    } else if !query_param.starttime.is_empty() {
        starttime_str = query_param.starttime.clone();
        starttime_str = starttime_str.replace("-","/").replace("T"," ");
        sql.push_str(" and ");
        sql.push_str(" datetime_off >= truetime.string2nominalf('");
        sql.push_str(&starttime_str);
        sql.push_str("')");
    } else if !query_param.endtime.is_empty() {
        endtime_str = query_param.endtime.clone();
        endtime_str = endtime_str.replace("-","/").replace("T"," ");
        sql.push_str(" and ");
        sql.push_str(" datetime_on <= truetime.string2nominalf('");
        sql.push_str(&endtime_str);
        sql.push_str("')");
    }
      sql.push_str(" order by net, sta, seedchan, location, samplerate, datetime_on, datetime_off");
}

pub fn create_str_from_list(user_input: String, db_col_name: String, a_str: &mut String){

   let a_list: Vec<&str> = user_input.split(',').collect();
   if a_list.len() == 1 {
       a_str.push_str(&db_col_name);
       if a_list[0].find('*') != None || a_list[0].find('?') != None {
              a_str.push_str(" like '");
	      a_str.push_str(&a_list[0].replace("*","%").replace("?","_").replace("-"," "));
	      a_str.push_str("%'");
       } else {
              a_str.push_str(" = '");
	      a_str.push_str(&a_list[0]);
	      a_str.push_str("'");
       }
   } else {
       for i in a_list.iter(){
	    if !a_str.is_empty() {
	        a_str.push_str(" or ");
	    }
	    a_str.push_str(&db_col_name);
	    if i.find('*') != None || i.find('?') != None {
	        a_str.push_str(" like '");
	        a_str.push_str(&i.replace("*","%").replace("?","_").replace("-"," "));	
	        a_str.push_str("%'");
	    } else {
	        a_str.push_str(" = '");
		a_str.push_str(&i);
		a_str.push_str("'");
	    }
	}
	}
}  

pub fn append_to_sql(str_to_append: &String, sql: &mut String){
    if sql.find("where") != None {
       sql.push_str(" and ")
    } else {
       sql.push_str(" where ");
    }
    sql.push_str(" ( ");
    //convert str_to_append to upper case, in case the user enters net, sta, chan or location in lowercase
    //sql.push_str(&(str_to_append.to_uppercase()));
    sql.push_str(&str_to_append.to_uppercase());  
    sql.push_str(" ) ");
}

// a_datetime = yyyy-mm-ddThh:mm:ss.ssssssssss, convert it to yyyy-mm-ddThh:mm:ss.sssss
// a_datetime = yyyy-mm-ddThh:mm:ss, convert it to yyyy-mm-ddThh:mm:ss.sssss
// a_datetime = yyyy-mm-dd, convert it to yyyy-mm-ddThh:mm:ss.sssss
pub fn format_datetime(a_datetime: &String) -> String {
    
    let mut out_datetime = a_datetime.clone();
    let mut temp = a_datetime.clone();
    if a_datetime.contains(".") {
        let v: Vec<&str> = temp.split(".").collect();
        if v[1].len() > 5 {
            out_datetime = v[0].to_string();
            let (keep, discard) = v[1].split_at(5);
            out_datetime.push_str(".");
            out_datetime.push_str(keep);
        }
        if v[1].len() < 5 {
            let mut v: Vec<&str> = temp.split(".").collect();                                                                                                          
            for _x in 0..5-v[1].len() {                                                                                                                                
                out_datetime.push_str("0");                                                                                                                                
            } 
        }
        
    } else if a_datetime.contains("T") {
        out_datetime.push_str(".00000");
    } else {
        out_datetime.push_str("T00:00:00.00000");
    }
    
    return out_datetime;
}

// Use for output. pad a_datetime with 0 till 6 decimal places and end with Z
pub fn pad_datetime(a_datetime: &String) -> String {
    let mut out_datetime = a_datetime.clone();
    let mut temp = a_datetime.clone();
    let v: Vec<&str> = temp.split(".").collect();
    for _x in 0..6-v[1].len() {
        out_datetime.push_str("0");
    }
    out_datetime.push_str("Z");
    return out_datetime;
}

pub fn write_headings(format: &String){
    if format == "text" {
        println! ("{n:<width$} {s:<widths$} {l:<widthl$} {c:<widthc$} {q:widthq$} {rate:<widthr$} {earliest:<widthe$} {latest:widthld$} {updated:widthu$} {timespans:widtht$} {restriction}", n="#Network", width=2, s="Station", widths=5, l="Location", widthl=2, c="Channel", widthc=3, q="Quality", widthq=1, rate="SampleRate", widthr=6, earliest="Earliest", widthe=8, latest="Latest", widthld=6, updated="Updated", widthu=4, timespans="TimeSpans", widtht=4, restriction="Restriction"); 
    } else if format == "geocsv" {
        println!("#dataset: GeoCSV 2.0");
        println!("#delimiter : |");
        println!("#field_unit : unitless | unitless | unitless | unitless | unitless | hertz | ISO_8601 | ISO_8601 | ISO_8601 | unitless | unitless");
        println!("#field_type : string | string | string | string | string | float | datetime | datetime | datetime | integer | string");
        println!("Network|Station|Location|Channel|Quality|SampleRate|Earliest|Latest|Updated|TimeSpans|Restriction");
    }
}

