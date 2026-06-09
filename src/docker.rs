use std::fs;
use std::path::Path;
use console::style;

pub fn add_docker_support(project_path: &Path, artifact: &str) -> Result<(), String> {
    println!("  {} {}", style("🐳").cyan(), style("Adding Docker support...").cyan());

    let dockerfile_content = format!(r#"# Build stage
FROM eclipse-temurin:21-jdk-alpine AS build
WORKDIR /app
COPY . .
RUN if [ -f "mvnw" ]; then ./mvnw clean package -DskipTests; else ./gradlew build -x test; fi
# Find the generated jar and copy it to a known location, ignoring plain jars
RUN find . -type f -name "*.jar" ! -name "*-plain.jar" -exec cp {{}} /app/app.jar \;

# Run stage
FROM eclipse-temurin:21-jre-alpine
WORKDIR /app
COPY --from=build /app/app.jar app.jar
EXPOSE 8080
ENTRYPOINT ["java", "-jar", "app.jar"]
"#);

    let compose_content = format!(r#"version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - SPRING_DATASOURCE_URL=jdbc:postgresql://db:5432/${{POSTGRES_DB}}
      - SPRING_DATASOURCE_USERNAME=${{POSTGRES_USER}}
      - SPRING_DATASOURCE_PASSWORD=${{POSTGRES_PASSWORD}}
    depends_on:
      db:
        condition: service_healthy

  db:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: ${{POSTGRES_USER}}
      POSTGRES_PASSWORD: ${{POSTGRES_PASSWORD}}
      POSTGRES_DB: ${{POSTGRES_DB}}
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${{POSTGRES_USER}} -d ${{POSTGRES_DB}}"]
      interval: 5s
      timeout: 5s
      retries: 5
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
"#);

    let env_content = format!(r#"POSTGRES_USER=myuser
POSTGRES_PASSWORD=secret
POSTGRES_DB={}
"#, artifact);

    fs::write(project_path.join("Dockerfile"), dockerfile_content).map_err(|e| e.to_string())?;
    fs::write(project_path.join("docker-compose.yml"), compose_content).map_err(|e| e.to_string())?;
    fs::write(project_path.join(".env"), env_content).map_err(|e| e.to_string())?;

    println!("  {} {}", style("✓").green(), style("Dockerfile, docker-compose.yml, and .env created.").dim());

    Ok(())
}
