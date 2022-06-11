
use diesel::{r2d2::ConnectionManager, SqliteConnection};
use r2d2::Pool;
use simple_jobs::{sqlite_job::DieselSqliteJob, Job, JobStatus};
use tokio::process::Command;
use uuid::Uuid;
use std::{time::Duration, ffi::OsStr, collections::HashMap};
use std::path::{PathBuf};
use crate::representations::TagentError;

pub async fn run_echo<J>(id: Uuid, job: J) -> Result<String, TagentError> 
where J: Job {
    // The usage is similar as with the standard library's `Command` type
    let mut child = Command::new("echo")
        .arg("hello")
        .arg("world")
        .spawn()
        .expect("failed to spawn");

    // Await until the command completes
    let status = child.wait().await?;
    println!("the command exited with: {}", status);
    Ok(format!("status was: {status}"))    
}

pub async fn echo_example(db_pool: &Pool<ConnectionManager<SqliteConnection>>) -> std::io::Result<()> {
    let job: DieselSqliteJob<String, TagentError> = DieselSqliteJob::new(db_pool);
    let id = job.submit(run_echo)?;
    let _info = job.load(id)?;
    let final_job = loop {
        let current_job = job.load(id)?;
        if current_job.status == JobStatus::Finished {
            break current_job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(final_job.status, JobStatus::Finished);
    dbg!(&final_job);
    Ok(())
}


// run a command line
pub async fn run_command(program: &OsStr, 
    args: &Vec<PathBuf>, 
    envs: &HashMap<String, String>, 
    current_dir: &OsStr,) 
    -> Result<String, TagentError> 
    {

    let output = Command::new(program)
        .args(args)
        .envs(envs)
        .current_dir(current_dir)
        .output()
        .await?;
    // return stdout for successfuly commands; otherwise, return stderr
    let status = output.status;
    if status.success() {
        let out = String::from_utf8(output.stdout)?;
        return Ok(out);
    }
    let out = String::from_utf8(output.stderr)?;
    Ok(out)
}

pub async fn env_example(db_pool: &Pool<ConnectionManager<SqliteConnection>>) -> std::io::Result<()> {
    let job: DieselSqliteJob<String, TagentError> = DieselSqliteJob::new(db_pool);
    let id = job.submit(|id, job | async move {
        let temp = tempfile::TempDir::new()?;
        let temp_dir = temp.path();
        let current_dir = OsStr::new(temp_dir);    
        let program = OsStr::new("env");
        let args: Vec<PathBuf> = Vec::new();
        let envs: HashMap<String, String> = HashMap::from([
            ("TAGENT_VAR".to_string(), "TAGENT VALUE".to_string(), )
        ]);
    
        let out = run_command(&program, &args, &envs, &current_dir).await?;
        Ok(out)
    })?;
    let _info = job.load(id)?;
    let final_job = loop {
        let current_job = job.load(id)?;
        if current_job.status == JobStatus::Finished {
            break current_job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(final_job.status, JobStatus::Finished);
    dbg!(&final_job.result);
    let result = final_job.result.expect("did not ger result from finished job")?;
    assert!(result.contains("TAGENT_VAR=TAGENT VALUE"));
    Ok(())
}

pub async fn pwd_example(db_pool: &Pool<ConnectionManager<SqliteConnection>>) -> std::io::Result<()> {
    let job: DieselSqliteJob<String, TagentError> = DieselSqliteJob::new(db_pool);
    let id = job.submit(|id, job | async move {
        let temp = tempfile::TempDir::new()?;
        let temp_dir = temp.path();
        let current_dir = OsStr::new(temp_dir);    
        let program = OsStr::new("pwd");
        let args: Vec<PathBuf> = Vec::new();
        let envs: HashMap<String, String> = HashMap::from([]);
        let out = run_command(&program, &args, &envs, &current_dir).await?;
        Ok(out)
    })?;
    let _info = job.load(id)?;
    let final_job = loop {
        let current_job = job.load(id)?;
        if current_job.status == JobStatus::Finished {
            break current_job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(final_job.status, JobStatus::Finished);
    dbg!(&final_job.result);
    let result = final_job.result.expect("did not ger result from finished job")?;
    assert!(result.contains("/tmp/"));
    Ok(())
}

pub async fn hello_example(name: &str) -> Result<String, TagentError> {
    Ok(format!("hello, {}", name))
}

pub async fn move_params_example(db_pool: &Pool<ConnectionManager<SqliteConnection>>) -> std::io::Result<()> {
    let job: DieselSqliteJob<String, TagentError> = DieselSqliteJob::new(db_pool);
    // with name as str, this code compiles
    // let name = "Walter";
    // with name as String, code fails to comile with error cannot move out of name, a captured variable in a Fn closure
    let name = String::from("Walter");
    let id = job.submit( |id, job | async move {
        let out = hello_example(&name).await?;
        Ok(out)
    })?;
    Ok(())
}    



#[cfg(test)]
mod test {
    use crate::db;

    use super::{echo_example, env_example, pwd_example, move_params_example};


    #[actix_rt::test]
    async fn test_move_params() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let db_name = format!("{}/test_echo_example.db", temp.path().to_string_lossy());
        let db_pool = db::get_db_pool(Some(String::from(db_name)));
        let _r = move_params_example(&db_pool).await?;

        Ok(())
    }

    #[actix_rt::test]
    async fn test_echo_example() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let db_name = format!("{}/test_echo_example.db", temp.path().to_string_lossy());
        let db_pool = db::get_db_pool(Some(String::from(db_name)));
        let _r = echo_example(&db_pool).await?;
        
        Ok(())
    }

    #[actix_rt::test]
    async fn test_env_example() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let db_name = format!("{}/test_echo_example.db", temp.path().to_string_lossy());
        let db_pool = db::get_db_pool(Some(String::from(db_name)));
        let _r = env_example(&db_pool).await;
        Ok(())
    }

    #[actix_rt::test]
    async fn test_pwd_example() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let db_name = format!("{}/test_pwd_example.db", temp.path().to_string_lossy());
        let db_pool = db::get_db_pool(Some(String::from(db_name)));
        let _r = pwd_example(&db_pool).await;

        Ok(())
    }


}