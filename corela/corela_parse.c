#include <ctype.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define CORELA_PARSE_EXIT_SUCCESS 0
#define CORELA_PARSE_EXIT_FAILURE 1

typedef struct {
  const char *before;
  const char *after;
} CorelaReplacement;

static int parse_file(const char *path, FILE *output, unsigned int depth) {
  if (depth >= 64) {
    fprintf(stderr, "%s: insertion nesting limit exceeded\n", path);
    return CORELA_PARSE_EXIT_FAILURE;
  }
  FILE *input = fopen(path, "r");
  if (input == NULL) {
    fprintf(stderr, "%s: cannot open source\n", path);
    return CORELA_PARSE_EXIT_FAILURE;
  }

  const CorelaReplacement replacements[] = {
      {"i32", "CORELA_I32"},         {"u32", "CORELA_U32"},
      {"and", "CORELA_AND"},         {"exit_success", "CORELA_EXIT_SUCCESS"},
      {"shift_is", "CORELA_IS"},     {"shift_0", "CORELA_SHIFT_0"},
      {"shift_1", "CORELA_SHIFT_1"}, {"or", "CORELA_SHIFT_OR"},
  };
  const size_t replacement_count =
      sizeof(replacements) / sizeof(replacements[0]);

  char line[4096];
  while (fgets(line, sizeof(line), input) != NULL) {
    // Ignore surrounding whitespace when checking for a bracket line.
    const char *start = line;
    while (isspace((unsigned char)*start)) {
      ++start;
    }

    static const char inject_prefix[] = "inject_raw file_from_current";
    const size_t prefix_length = sizeof(inject_prefix) - 1;
    if (strncmp(start, inject_prefix, prefix_length) == 0 &&
        isspace((unsigned char)start[prefix_length])) {
      const char *filename = start + prefix_length;
      while (isspace((unsigned char)*filename)) {
        ++filename;
      }
      const char *end = filename + strlen(filename);
      while (end > filename && isspace((unsigned char)end[-1])) {
        --end;
      }
      const char *slash = strrchr(path, '/');
      const size_t directory_length =
          slash == NULL ? 0 : (size_t)(slash - path) + 1;
      const size_t filename_length = (size_t)(end - filename);
      char injected_path[4096];
      if (filename_length == 0 ||
          directory_length + filename_length >= sizeof(injected_path)) {
        fprintf(stderr, "%s: invalid injection path\n", path);
        fclose(input);
        return CORELA_PARSE_EXIT_FAILURE;
      }
      memcpy(injected_path, path, directory_length);
      memcpy(injected_path + directory_length, filename, filename_length);
      injected_path[directory_length + filename_length] = '\0';
      if (parse_file(injected_path, output, depth + 1) !=
          CORELA_PARSE_EXIT_SUCCESS) {
        fclose(input);
        return CORELA_PARSE_EXIT_FAILURE;
      }
      fputc('\n', output);
      continue;
    }

    // An include directive starts at the first non-whitespace character.
    if (strncmp(start, "include", 7) == 0 &&
        (isspace((unsigned char)start[7]) || start[7] == '<' ||
         start[7] == '"')) {
      fwrite(line, 1, (size_t)(start - line), output);
      fputc('#', output);
      fputs(start, output);
      continue;
    }

    const char *trimmed_end = start + strlen(start);
    while (trimmed_end > start && isspace((unsigned char)trimmed_end[-1])) {
      --trimmed_end;
    }

    if (trimmed_end - start == 10 && strncmp(start, "sokol_main", 10) == 0) {
      fwrite(line, 1, (size_t)(start - line), output);
      fputs("sapp_desc sokol_main(int argc, char *argv[])", output);
      fputs(trimmed_end, output);
      continue;
    }

    if (trimmed_end - start == 2 &&
        (strncmp(start, "di", 2) == 0 || strncmp(start, "do", 2) == 0)) {
      fwrite(line, 1, (size_t)(start - line), output);
      fputc(start[1] == 'i' ? '{' : '}', output);
      fputs(trimmed_end, output);
      continue;
    }

    const char *position = line;
    while (*position != '\0') {
      const char *nearest = NULL;
      const CorelaReplacement *selected = NULL;

      for (size_t i = 0; i < replacement_count; ++i) {
        const char *found = strstr(position, replacements[i].before);
        while (found != NULL) {
          const char *end = found + strlen(replacements[i].before);
          if ((found == line ||
               (!isalnum((unsigned char)found[-1]) && found[-1] != '_')) &&
              !isalnum((unsigned char)*end) && *end != '_') {
            break;
          }
          found = strstr(found + 1, replacements[i].before);
        }
        if (found != NULL && (nearest == NULL || found < nearest)) {
          nearest = found;
          selected = &replacements[i];
        }
      }

      if (selected == NULL) {
        fputs(position, output);
        break;
      }

      fwrite(position, 1, (size_t)(nearest - position), output);
      fputs(selected->after, output);
      position = nearest + strlen(selected->before);
    }
  }

  const int read_failed = ferror(input);
  fclose(input);
  return read_failed ? CORELA_PARSE_EXIT_FAILURE : CORELA_PARSE_EXIT_SUCCESS;
}

int main(void) {
  FILE *output = fopen("main.c", "w");
  if (output == NULL) {
    fprintf(stderr, "-main.c\n");
    return CORELA_PARSE_EXIT_FAILURE;
  }
  const int result = parse_file("corela/main.corela", output, 0);
  const int write_failed = ferror(output);
  const int close_failed = fclose(output);
  if (result != CORELA_PARSE_EXIT_SUCCESS || write_failed || close_failed) {
    return CORELA_PARSE_EXIT_FAILURE;
  }
  printf("+main.c\n");
  return CORELA_PARSE_EXIT_SUCCESS;
}
